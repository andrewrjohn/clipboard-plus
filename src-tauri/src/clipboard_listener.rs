extern crate clipboard_master;

use chrono::Utc;
use clipboard_master::{CallbackResult, ClipboardHandler};
use diesel::prelude::*;
use std::{io, sync::Mutex};
use tauri::{AppHandle, Manager};
use tauri_plugin_clipboard_manager::ClipboardExt;

use crate::{
    dispatch_clipboard_change, get_source_app, image_to_base64,
    models::{Item, NewItem},
    schema, AppState,
};

pub struct ClipboardListener {
    pub app_handle: AppHandle,
}

impl ClipboardHandler for ClipboardListener {
    fn on_clipboard_change(&mut self) -> CallbackResult {
        use crate::schema::items::dsl::*;

        let state = self.app_handle.state::<Mutex<AppState>>();
        let state = state.lock().unwrap();

        let connection = &mut *state.conn.lock().unwrap();

        let source = get_source_app();

        if let Ok(str) = self.app_handle.clipboard().read_text() {
            if items
                .filter(text.eq(&str))
                .first::<Item>(connection)
                .is_ok()
            {
                // Move the item to the top of the list
                diesel::update(items)
                    .set(timestamp.eq(Utc::now().timestamp_millis()))
                    .filter(text.eq(&str))
                    .execute(connection)
                    .expect("Failed to update timestamp");

                dispatch_clipboard_change(self.app_handle.clone());

                return CallbackResult::Next;
            }

            let new_item = NewItem {
                text: Some(&str),
                image: None,
                image_width: None,
                image_height: None,
                timestamp: Utc::now().timestamp_millis(),
                size_bytes: str.len() as i32,
                source_app: source.as_deref(),
            };

            diesel::insert_into(schema::items::table)
                .values(&new_item)
                .execute(connection)
                .expect("Failed to insert clipboard item");
        };

        if let Ok(img) = self.app_handle.clipboard().read_image() {
            let image_url = image_to_base64(img.clone());

            if items
                .filter(image.eq(&image_url))
                .first::<Item>(connection)
                .is_ok()
            {
                // Move the item to the top of the list
                diesel::update(items)
                    .set(timestamp.eq(Utc::now().timestamp_millis()))
                    .filter(image.eq(&image_url))
                    .execute(connection)
                    .expect("Failed to update timestamp");
                return CallbackResult::Next;
            }

            let new_item = NewItem {
                text: None,
                image: Some(&image_url),
                image_width: Some(img.clone().width() as i32),
                image_height: Some(img.clone().height() as i32),
                timestamp: Utc::now().timestamp_millis(),
                size_bytes: image_url.len() as i32,
                source_app: source.as_deref(),
            };

            diesel::insert_into(schema::items::table)
                .values(&new_item)
                .execute(connection)
                .expect("Failed to insert clipboard item");
        }

        dispatch_clipboard_change(self.app_handle.clone());
        CallbackResult::Next
    }

    fn on_clipboard_error(&mut self, error: io::Error) -> CallbackResult {
        eprintln!("Error: {}", error);
        CallbackResult::Next
    }
}
