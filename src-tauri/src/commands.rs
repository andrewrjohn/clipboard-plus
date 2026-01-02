extern crate clipboard_master;

use base64::{engine::general_purpose, Engine};
use chrono::Utc;
use diesel::{dsl::sum, prelude::*};
use enigo::{Enigo, Key, KeyboardControllable};
use image as image_lib;
use std::sync::Mutex;

use tauri::{image::Image, AppHandle, Manager, State};
use tauri_plugin_clipboard_manager::ClipboardExt;

use crate::{dispatch_clipboard_change, get_db_path, models::Item, AppState, SystemData};

#[tauri::command]
#[specta::specta]
pub fn get_clipboard_history(state: State<'_, Mutex<AppState>>) -> Vec<Item> {
    use crate::schema::items::dsl::*;
    let state = state.lock().unwrap();

    let connection = &mut *state.conn.lock().unwrap();

    let results = items
        .order(timestamp.desc())
        .load::<Item>(connection)
        .expect("Failed to load clipboard items");

    return results;
}

#[tauri::command]
#[specta::specta]
pub fn copy(app_handle: AppHandle, _timestamp: i64) -> i64 {
    use crate::schema::items::dsl::*;

    let state = app_handle.state::<Mutex<AppState>>();
    let state = state.lock().unwrap();

    let connection = &mut *state.conn.lock().unwrap();

    let item = items
        .filter(timestamp.eq(_timestamp))
        .first::<Item>(connection)
        .expect("Failed to find item");

    if let Some(t) = &item.text {
        app_handle.clipboard().write_text(t).unwrap();
    } else if let (Some(img), Some(img_width), Some(img_height)) =
        (&item.image, &item.image_width, &item.image_height)
    {
        // Remove the "data:image/png;base64," prefix if present
        let base64_data = if let Some(stripped) = img.strip_prefix("data:image/png;base64,") {
            stripped
        } else {
            img.as_str()
        };

        // Convert base64 to bytes
        let decoded_bytes = match general_purpose::STANDARD.decode(base64_data) {
            Ok(bytes) => bytes,
            Err(e) => {
                eprintln!("Failed to decode base64 image: {}", e);
                return 0;
            }
        };

        // Decode PNG bytes to get raw RGBA pixel data
        let img = match image_lib::load_from_memory(&decoded_bytes) {
            Ok(img) => img.to_rgba8(),
            Err(e) => {
                eprintln!("Failed to decode PNG image: {}", e);
                return 0;
            }
        };

        // Extract RGBA pixel data
        let rgba_data = img.as_raw();

        // Create a Tauri Image from RGBA pixel data
        let tauri_image = Image::new(rgba_data, *img_width as u32, *img_height as u32);

        // Write image to clipboard using the clipboard plugin
        if let Err(e) = app_handle.clipboard().write_image(&tauri_image) {
            eprintln!("Failed to write image to clipboard: {}", e);
        }
    }

    let new_timestamp = Utc::now().timestamp_millis();

    diesel::update(items)
        .set(timestamp.eq(new_timestamp))
        .filter(timestamp.eq(_timestamp))
        .execute(connection)
        .expect("Failed to update timestamp");

    dispatch_clipboard_change(app_handle.clone());

    return new_timestamp;
}

#[tauri::command]
#[specta::specta]
pub fn paste() {
    let mut enigo = Enigo::new();
    enigo.key_down(Key::Meta);
    enigo.key_down(Key::Layout('v'));
    enigo.key_up(Key::Layout('v'));
    enigo.key_up(Key::Meta);
}

#[tauri::command]
#[specta::specta]
pub fn clear_clipboard(app_handle: AppHandle) {
    use crate::schema::items::dsl::*;

    let state = app_handle.state::<Mutex<AppState>>();
    let state = state.lock().unwrap();
    let connection = &mut *state.conn.lock().unwrap();

    diesel::delete(items)
        .execute(connection)
        .expect("Failed to clear clipboard");

    dispatch_clipboard_change(app_handle.clone());
}

#[tauri::command]
#[specta::specta]
pub fn clean_old_items(app_handle: AppHandle, days: i32) {
    use crate::schema::items::dsl::*;

    let state = app_handle.state::<Mutex<AppState>>();
    let state = state.lock().unwrap();

    let connection = &mut *state.conn.lock().unwrap();

    let old_items_timestamp = Utc::now().timestamp_millis() - (days as i64 * 24 * 60 * 60 * 1000);

    diesel::delete(items)
        .filter(timestamp.lt(old_items_timestamp))
        .execute(connection)
        .expect("Failed to clear clipboard");

    dispatch_clipboard_change(app_handle.clone());
}

#[tauri::command]
#[specta::specta]
pub fn delete_clipboard_item(app_handle: AppHandle, _timestamp: i64) {
    use crate::schema::items::dsl::*;

    let state = app_handle.state::<Mutex<AppState>>();
    let state = state.lock().unwrap();

    let connection = &mut *state.conn.lock().unwrap();

    diesel::delete(items)
        .filter(timestamp.eq(_timestamp))
        .execute(connection)
        .expect("Failed to delete clipboard item");

    dispatch_clipboard_change(app_handle.clone());
}

#[tauri::command]
#[specta::specta]
pub fn get_system_data(app_handle: AppHandle) -> SystemData {
    use crate::schema::items::dsl::*;

    let state = app_handle.state::<Mutex<AppState>>();
    let state = state.lock().unwrap();

    let connection = &mut *state.conn.lock().unwrap();

    let size = items
        .select(sum(size_bytes).nullable())
        .first(connection)
        .unwrap_or(Some(0));

    let db_path = get_db_path(app_handle.clone())
        .to_string_lossy()
        .to_string();

    SystemData {
        size_bytes: size.unwrap_or_default() as usize,
        db_path,
    }
}
