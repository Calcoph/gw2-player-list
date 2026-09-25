use const_format::formatcp;

use crate::{State, VERSION_MAJOR, VERSION_MINOR, VERSION_PATCH, log};

pub fn check_for_updates(state: &mut State) -> Option<String> {
    const RELEASE_LIST_LEN: u32 = 5; // check at most the most recent RELEASE_LIST_LEN updates
    const URL: &'static str = formatcp!("https://api.github.com/repos/Calcoph/gw2-player-list/releases?per_page={RELEASE_LIST_LEN}&page=1");

    const USER_AGENT: &'static str = formatcp!("gw2_player_list_{VERSION_MAJOR}_{VERSION_MINOR}_{VERSION_PATCH}");

    if !state.config.auto_check_update {
        return None;
    }

    let http_client = reqwest::blocking::ClientBuilder::new()
        .user_agent(USER_AGENT)
        .build().ok()?;

    let response = http_client.get(URL).send().ok()?; // TODO: Do not spam the api, do not check for updates every time gw2 is launched. Maybe once a week or so. Also use etags https://docs.github.com/en/rest/using-the-rest-api/best-practices-for-using-the-rest-api?apiVersion=2026-03-10#make-requests-that-can-be-cached

    let status = response.status();
    let body = response.text().ok()?;
    if !status.is_success() {
        log(&format!("Update error ({}) response body: {body}", status.as_u16()));
        return None;
    }

    let ret: serde_json::Value = serde_json::from_str(&body).ok()?;
    let serde_json::Value::Array(releases) = ret else {
        return None;
    };

    let mut chosen_release = None;
    'outer: for release in releases {
        let serde_json::Value::Object(release) = release else {
            continue;
        };

        if !state.config.auto_check_beta {
            if let Some(serde_json::Value::Bool(true)) = release.get("prerelease") {
                continue;
            }
        }

        let Some(serde_json::Value::Array(assets)) = release.get("assets") else {
            continue;
        };

        let Some(serde_json::Value::String(tag)) = release.get("tag_name") else {
            continue;
        };

        let Some(version) = parse_tag(tag) else {
            continue;
        };

        if !is_version_newer(version) {
            continue;
        }

        for asset in assets {
            let Some(serde_json::Value::String(name)) = asset.get("name") else {
                continue;
            };

            if name != "player_list.dll" {
                continue;
            }

            let Some(serde_json::Value::String(url)) = asset.get("browser_download_url") else {
                continue;
            };

            log(&format!("Updating version to {}.{}.{}", version.0, version.1, version.2));
            chosen_release = Some(url.clone());
            break 'outer;
        }
    }

    if chosen_release.is_none() {
        log("No new version has been detected");
    }
    chosen_release
}

fn is_version_newer((major, minor, patch): (u32, u32, u32)) -> bool {
    if major > VERSION_MAJOR {
        return true;
    }
    if major < VERSION_MAJOR {
        return false;
    }

    if minor > VERSION_MINOR {
        return true;
    }
    if minor < VERSION_MINOR {
        return false;
    }

    return patch > VERSION_PATCH;
}

fn parse_tag(tag: &str) -> Option<(u32, u32, u32)> {
    if !tag.starts_with("v") {
        return None;
    }

    let tag = tag.trim_start_matches("v");

    let mut parts = tag.split(".");
    let major = parts.next()?;
    let minor = parts.next()?;
    let patch = parts.next()?;

    let patch = patch.split("-").next()?;

    let major = major.parse().ok()?;
    let minor = minor.parse().ok()?;
    let patch = patch.parse().ok()?;

    Some((major, minor, patch))
}
