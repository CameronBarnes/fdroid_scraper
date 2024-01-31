use once_cell::sync::Lazy;
use rayon::prelude::{ParallelBridge, ParallelIterator};
use regex::Regex;
use reqwest::blocking::Client;

use crate::types::{Category, Document, DownloadType, LibraryItem};

#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
fn parse_size(size: &str) -> u64 {
    let (size, unit) = size.split_once(' ').unwrap();
    let mut size: f64 = size.parse().unwrap();
    if unit.eq_ignore_ascii_case("KiB") {
        size *= 1024.0;
    } else if unit.eq_ignore_ascii_case("MiB") {
        size *= 1_048_576.0;
    } else if unit.eq_ignore_ascii_case("GiB") {
        size *= 1_073_741_824.0;
    }
    size as u64
}

pub fn parse_fdroid() -> LibraryItem {
    static RE: Lazy<Regex> = Lazy::new(|| {
        Regex::new("<p><a href=\"/en/categories/(.+?)/\">Show all \\d+ packages</a></p>").unwrap()
    });

    let raw_page = get_page_from_path("https://f-droid.org/en/packages/");
    let results = RE
        .captures_iter(&raw_page)
        .map(|e| e.extract())
        .map(|(_, [name])| {
            eprintln!("{name}");
            parse_category(name)
        })
        .collect();

    LibraryItem::Category(Category::new(String::from("F-Droid"), results, false))
}

fn parse_category(name: &str) -> LibraryItem {
    let category_path = format!("https://f-droid.org/en/categories/{name}/");
    let mut raw_page = get_page_from_path(&category_path);
    let mut items = parse_category_page(&raw_page);

    // Get items from subsequent pages
    let mut counter = 1;
    while !raw_page.contains("<li class=\"nav next disabled\">") {
        counter += 1;
        eprintln!("{name}: {counter}");
        let path = format!("{category_path}{counter}/");
        raw_page = get_page_from_path(&path);
        items.append(&mut parse_category_page(&raw_page));
    }

    LibraryItem::Category(Category::new(name.to_string(), items, false))
}

fn parse_category_page(input: &str) -> Vec<LibraryItem> {
    static RE: Lazy<Regex> =
        Lazy::new(|| Regex::new("<a class=\"package-header\" href=\"(.+?)\">").unwrap());

    // we want to make sure to exclude the recommended apps that arent actually in this category
    let input = input.split_once("<h3>Last Updated</h3>").unwrap().0;

    RE.captures_iter(input)
        .map(|e| e.extract())
        .map(|(_, [url])| {
            let path = format!("https://f-droid.org{url}");
            parse_item(&get_page_from_path(&path))
        })
        .collect()
}

fn parse_item(input: &str) -> LibraryItem {
    static RE: Lazy<Regex> = Lazy::new(|| {
        Regex::new("<title>(.+?) \\| .+?</title>[\\s\\S]+?<p class=\"package-version-download\">\\s+<b>\\s+<a href=\"(.+?)\">\\s+Download APK\\s+</a>\\s+</b>\\s+(.+?)\n").unwrap()
    });

    let (_, [title, url, size]) = RE.captures_iter(input).next().unwrap().extract();

    let size = parse_size(size);

    LibraryItem::Document(Document::new(
        title.to_string(),
        url.to_string(),
        size,
        DownloadType::Http,
    ))
}

pub fn get_page_from_path(path: &str) -> String {
    static CLIENT: Lazy<Client> = Lazy::new(|| {
        reqwest::blocking::ClientBuilder::new()
            .user_agent("Mozilla/5.0 (X11; Linux x86_64; rv:109.0) Gecko/20100101 Firefox/117.0")
            .build()
            .unwrap()
    });
    CLIENT.get(path).send().unwrap().text().unwrap()
}
