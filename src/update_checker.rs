use std::time::{Duration, UNIX_EPOCH};

use const_format::formatcp;

use crate::{State, VERSION_MAJOR, VERSION_MINOR, VERSION_PATCH, config::AvailableVersion, log};

pub fn use_cached_version(state: &mut State) -> Option<String> {
    if !state.config.auto_check_update {
        return None;
    }

    if !state.config.update_permitted {
        return None;
    }

    let url = state.updater_data.available_version.take()?.url;
    state.config.update_permitted = false;

    Some(url)
}

pub fn check_for_updates(state: &mut State) {
    const RELEASE_LIST_LEN: u32 = 5; // check at most the most recent RELEASE_LIST_LEN updates
    const URL: &'static str = formatcp!("https://api.github.com/repos/Calcoph/gw2-player-list/releases?per_page={RELEASE_LIST_LEN}&page=1");

    const USER_AGENT: &'static str = formatcp!("gw2_player_list_{VERSION_MAJOR}_{VERSION_MINOR}_{VERSION_PATCH}");

    if !state.config.auto_check_update {
        return;
    }

    let now = UNIX_EPOCH.elapsed().unwrap_or(Duration::from_secs(0));
    let last_update = Duration::from_secs(state.updater_data.last_update_timestamp);
    let since_last_update = now - last_update;
    if since_last_update.as_secs() < state.updater_data.days_between_polls as u64 * 3600 * 24 {
        log("not updating since last update was not long ago enough");
        return;
    }

    let Ok(http_client) = reqwest::blocking::ClientBuilder::new()
        .user_agent(USER_AGENT)
        .build()
    else {
        return;
    };

    let request = http_client.get(URL);
    let request = if let Some(last_modified) = &state.updater_data.last_modified {
        request.header("if-modified-since", last_modified)
    } else {
        request
    };
    let request = if let Some(etag) = &state.updater_data.etag {
        request.header("if-none-match", etag)
    } else {
        request
    };

    let Ok(mut response) = request.send() else {
        return;
    };
    log(&format!("updater response: {response:?}"));

    let status = response.status();
    let headers = response.headers_mut();
    let last_modified = headers.remove("last-modified");
    let etag = headers.remove("etag");
    let Ok(body) = response.text() else {
        return;
    };
    log(&format!("updater response body: {body:?}"));
    if status.as_u16() == 304 { // Not Modified
        log("not updating since nothing changed");
        return;
    } else if !status.is_success() {
        log(&format!("Update error ({}) response body: {body}", status.as_u16()));
        return;
    }

    if let Some(last_modified) = last_modified {
        if let Ok(last_modified) = last_modified.to_str() {
            state.updater_data.last_modified = Some(last_modified.to_owned())
        }
    }

    if let Some(etag) = etag {
        if let Ok(etag) = etag.to_str() {
            state.updater_data.etag = Some(etag.to_owned())
        }
    }
    state.updater_data.last_update_timestamp = now.as_secs();

    let Ok(ret): Result<serde_json::Value, _> = serde_json::from_str(&body) else {
        return;
    };
    let serde_json::Value::Array(releases) = ret else {
        return;
    };

    let chosen_release = choose_release(state, releases);

    if let Some(version) = &chosen_release {
        log("New version available");
        state.updater_data.available_version = Some(version.clone())
    } else {
        log("No new version has been detected");
    }
}

fn choose_release(state: &State, releases: Vec<serde_json::Value>) -> Option<AvailableVersion> {
    for release in releases {
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
            return Some(AvailableVersion {
                major: version.0,
                minor: version.1,
                patch: version.2,
                url: url.clone(),
            });
        }
    }

    None
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
