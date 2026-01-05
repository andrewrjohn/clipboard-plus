#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

use crate::clipboard_listener::ClipboardListener;
use crate::models::Item;
use crate::schema::items;
use clipboard_master::Master;
use diesel::prelude::*;
use diesel::SqliteConnection;
use image::{DynamicImage, ImageBuffer, ImageFormat};
use serde::Serialize;
use specta::Type;
use specta_typescript::Typescript;
use std::fs::File;
use std::io::Write;
use std::{io::Cursor, path::PathBuf, sync::Mutex};
use tauri::{
    image::Image,
    path::BaseDirectory,
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    AppHandle, Manager,
};
use tauri_plugin_global_shortcut::{Code, GlobalShortcutExt, Modifiers, Shortcut};
use tauri_plugin_positioner::{Position, WindowExt};
use tauri_specta::collect_commands;
use tauri_specta::collect_events;
use tauri_specta::Builder;
use tauri_specta::Event;

pub mod clipboard_listener;
pub mod commands;
pub mod events;
pub mod models;
pub mod schema;

pub struct AppState {
    conn: Mutex<SqliteConnection>,
}

impl AppState {
    pub fn new(conn: SqliteConnection) -> Self {
        Self {
            conn: Mutex::new(conn),
        }
    }
}

fn establish_connection(app_handle: AppHandle) -> SqliteConnection {
    let db_url = get_db_path(app_handle).to_string_lossy().to_string();

    return SqliteConnection::establish(&db_url)
        .unwrap_or_else(|_| panic!("Error connecting to {}", db_url));
}

fn get_source_app() -> Option<String> {
    if let Ok(active_win) = active_win_pos_rs::get_active_window() {
        return Some(active_win.app_name);
    }
    return None;
}

pub struct SaveImageResult {
    pub path: String,
    pub width: u32,
    pub height: u32,
    pub size: usize,
    pub already_exists: bool,
}

/** Returns `None` if the image already exists */
pub fn save_image_to_file(
    app_handle: AppHandle,
    connection: &mut SqliteConnection,
    image: Image,
) -> SaveImageResult {
    let images_dir = app_handle.path().app_data_dir().unwrap().join("images");
    std::fs::create_dir_all(&images_dir).unwrap();

    let raw_image = image.rgba().to_vec();

    let hash = blake3::hash(&raw_image);

    let image_path = images_dir.join(format!("{}.png", hash.to_string()));

    if image_path.exists() {
        let item = schema::items::dsl::items
            .filter(items::image.eq(&image_path.to_string_lossy().to_string()))
            .first::<Item>(connection)
            .expect("Failed to find item");

        return SaveImageResult {
            path: item.image.unwrap(),
            width: item.image_width.unwrap() as u32,
            height: item.image_height.unwrap() as u32,
            size: item.size_bytes as usize,
            already_exists: true,
        };
    }

    let img_buffer = ImageBuffer::from_raw(image.width(), image.height(), raw_image.clone())
        .expect("Failed to create image buffer");
    let dynamic_image = DynamicImage::ImageRgba8(img_buffer);

    let mut png_bytes: Vec<u8> = Vec::new();

    dynamic_image
        .write_to(&mut Cursor::new(&mut png_bytes), ImageFormat::Png)
        .unwrap();

    let mut file = File::create(&image_path).expect("Failed to create file");
    file.write_all(&png_bytes)
        .expect("Failed to write image to file");

    return SaveImageResult {
        path: image_path.to_string_lossy().to_string(),
        width: image.width(),
        height: image.height(),
        size: png_bytes.len(),
        already_exists: false,
    };
}

/**
 * Dispatches a clipboard changed event
 */
fn dispatch_clipboard_change(app_handle: AppHandle) {
    events::ClipboardChangedEvent()
        .emit(&app_handle)
        .expect("Failed to emit clipboard changed event");
}

fn focus_main_window(app_handle: AppHandle) {
    if let Some(window) = app_handle.get_webview_window("main") {
        let _ = window.unminimize();
        let _ = window.show();
        let _ = window.set_focus();
    }

    let win = app_handle.get_webview_window("main").unwrap();
    let _ = win.as_ref().window().move_window(Position::Center);
}

fn get_db_path(app_handle: AppHandle) -> PathBuf {
    let dur = app_handle.path().local_data_dir().unwrap().join("CB Utils");
    std::fs::create_dir_all(&dur).unwrap();
    dur.join("clipboard.db")
}

#[derive(Debug, Serialize, Type)]
pub struct SystemData {
    pub size_bytes: usize,
    pub db_path: String,
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let builder = Builder::<tauri::Wry>::new()
        .commands(collect_commands![
            commands::get_clipboard_history,
            commands::copy,
            commands::delete_clipboard_item,
            commands::clear_clipboard,
            commands::clean_old_items,
            commands::paste,
            commands::get_system_data,
        ])
        .events(collect_events![events::ClipboardChangedEvent]);

    #[cfg(debug_assertions)]
    builder
        .export(
            Typescript::default().bigint(specta_typescript::BigIntExportBehavior::BigInt),
            "../src/bindings.ts",
        )
        .expect("Failed to export typescript bindings");

    let ctrl_cmd_c_global_shortcut =
        Shortcut::new(Some(Modifiers::CONTROL | Modifiers::SUPER), Code::KeyC);

    tauri::Builder::default()
        .invoke_handler(builder.invoke_handler())
        .plugin(tauri_plugin_positioner::init())
        .plugin(tauri_plugin_clipboard_manager::init())
        .plugin(tauri_plugin_opener::init())
        .setup(move |app| {
            let dir = get_db_path(app.handle().clone());
            println!("Database path: {}", dir.display());

            builder.mount_events(app);

            let conn = establish_connection(app.handle().clone());

            app.manage(Mutex::new(AppState::new(conn)));

            app.set_activation_policy(tauri::ActivationPolicy::Accessory);

            TrayIconBuilder::new()
                .icon(
                    Image::from_path(
                        app.path()
                            .resolve("icons/tray-icon.png", BaseDirectory::Resource)
                            .unwrap(),
                    )
                    .unwrap(),
                )
                .icon_as_template(true)
                .on_tray_icon_event(|tray, event| match event {
                    TrayIconEvent::Click {
                        button: MouseButton::Left,
                        button_state: MouseButtonState::Up,
                        ..
                    } => {
                        focus_main_window(tray.app_handle().to_owned());
                    }
                    _ => {}
                })
                .build(app)?;

            app.handle().plugin(
                tauri_plugin_global_shortcut::Builder::new()
                    .with_handler(move |app, shortcut, _event| {
                        if shortcut == &ctrl_cmd_c_global_shortcut {
                            focus_main_window(app.app_handle().to_owned());
                        }
                    })
                    .build(),
            )?;

            app.global_shortcut().register(ctrl_cmd_c_global_shortcut)?;

            let handler = ClipboardListener {
                app_handle: app.handle().clone(),
            };
            std::thread::spawn(move || {
                let mut master = Master::new(handler).expect("Failed to create clipboard master");
                master.run().expect("Failed to run clipboard master");
            });

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
