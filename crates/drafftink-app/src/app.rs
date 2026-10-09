//! Core application state and lifecycle.

use drafftink_core::canvas::Canvas;
use drafftink_core::collaboration::CollaborationManager;
use drafftink_core::input::InputState;
use drafftink_core::shapes::Shape;
#[cfg(target_arch = "wasm32")]
use drafftink_core::shapes::ShapeId;
use drafftink_core::shapes::ShapeTrait;
use drafftink_core::sync::{AwarenessState, ConnectionState, SyncEvent};
use drafftink_core::tools::{EraserMode, ToolKind};
#[cfg(not(target_arch = "wasm32"))]
use drafftink_render::PngRenderResult;
use drafftink_render::{
    AngleSnapInfo, GridStyle, RenderContext, Renderer, TextEditResult, TextEditState, TextKey,
    TextModifiers, VelloRenderer,
};
use kurbo::{Point, Size, Vec2};
use peniko::Color;
use std::sync::Arc;
use vello::util::RenderSurface;
use vello::wgpu::PresentMode;
use vello::{AaConfig, RenderParams, RendererOptions, Scene};
use winit::application::ApplicationHandler;
#[cfg(not(target_arch = "wasm32"))]
use winit::dpi::LogicalSize;
use winit::event::{ElementState, MouseButton, MouseScrollDelta, WindowEvent};
use winit::event_loop::{ActiveEventLoop, EventLoop};
use winit::keyboard::{Key, KeyCode, NamedKey, PhysicalKey};
use winit::window::{CursorIcon, Window, WindowId};

use crate::event_handler::EventHandler;
use crate::ui::{MathEditorState, SelectedShapeProps, UiAction, UiState, render_ui};
#[cfg(not(target_arch = "wasm32"))]
use std::time::Instant as FrameInstant;
#[cfg(target_arch = "wasm32")]
use web_time::Instant as FrameInstant;

#[cfg(feature = "native")]
pub mod file_ops {
    use drafftink_core::canvas::CanvasDocument;
    use std::sync::Mutex;

    // Channel for receiving async file operation results
    static PENDING_DOCUMENT: Mutex<Option<CanvasDocument>> = Mutex::new(None);
    static PENDING_LIBRARY: Mutex<Option<(String, CanvasDocument)>> = Mutex::new(None);

    /// Save document to a JSON file using native file dialog (async, non-blocking).
    pub fn save_document(document: &CanvasDocument, name: &str) {
        let doc = document.clone();
        let default_name = format!("{}.json", name);
        std::thread::spawn(move || {
            let dialog = rfd::FileDialog::new()
                .set_title("Save Document")
                .set_file_name(&default_name)
                .add_filter("DrafftInk Document", &["json"]);

            if let Some(path) = dialog.save_file() {
                match doc.to_json() {
                    Ok(json) => {
                        if let Err(e) = std::fs::write(&path, &json) {
                            log::error!("Failed to write file: {}", e);
                        } else {
                            log::info!("Saved document to: {:?}", path);
                        }
                    }
                    Err(e) => log::error!("Failed to serialize document: {}", e),
                }
            }
        });
    }

    /// Load document from a JSON file using native file dialog (async, non-blocking).
    /// Use `take_pending_document()` to retrieve the result.
    pub fn load_document() {
        std::thread::spawn(move || {
            let dialog = rfd::FileDialog::new()
                .set_title("Open Document")
                .add_filter("DrafftInk Document", &["json"])
                .add_filter("Excalidraw", &["excalidraw"]);

            if let Some(path) = dialog.pick_file() {
                match std::fs::read_to_string(&path) {
                    Ok(content) => {
                        let is_excalidraw = path
                            .extension()
                            .map(|e| e.to_string_lossy().to_lowercase() == "excalidraw")
                            .unwrap_or(false);

                        let result = if is_excalidraw {
                            CanvasDocument::from_excalidraw(&content).map_err(|e| e.to_string())
                        } else {
                            CanvasDocument::from_json(&content).map_err(|e| e.to_string())
                        };

                        match result {
                            Ok(doc) => {
                                log::info!("Loaded document from: {:?}", path);
                                if let Ok(mut pending) = PENDING_DOCUMENT.lock() {
                                    *pending = Some(doc);
                                }
                            }
                            Err(e) => log::error!("Failed to parse document: {}", e),
                        }
                    }
                    Err(e) => log::error!("Failed to read file: {}", e),
                }
            }
        });
    }

    /// Take pending document from async load operation.
    pub fn take_pending_document() -> Option<CanvasDocument> {
        PENDING_DOCUMENT.lock().ok().and_then(|mut p| p.take())
    }

    /// Load an Excalidraw library (`.excalidrawlib`) via native file dialog
    /// (async, non-blocking). Retrieve via `take_pending_library()`.
    pub fn load_excalidrawlib_async() {
        std::thread::spawn(move || {
            let dialog = rfd::FileDialog::new()
                .set_title("Open Excalidraw Library")
                .add_filter("Excalidraw Library", &["excalidrawlib"]);
            if let Some(path) = dialog.pick_file() {
                match std::fs::read_to_string(&path) {
                    Ok(content) => {
                        let name = path
                            .file_stem()
                            .map(|s| s.to_string_lossy().to_string())
                            .unwrap_or_else(|| "Library".to_string());
                        match super::build_library_document(&content, &name) {
                            Some(result) => {
                                if let Ok(mut pending) = PENDING_LIBRARY.lock() {
                                    *pending = Some(result);
                                }
                            }
                            None => log::error!("Not a valid Excalidraw library: {:?}", path),
                        }
                    }
                    Err(e) => log::error!("Failed to read library: {}", e),
                }
            }
        });
    }

    /// Take a pending library `(name, document)` from an async load.
    pub fn take_pending_library() -> Option<(String, CanvasDocument)> {
        PENDING_LIBRARY.lock().ok().and_then(|mut p| p.take())
    }

    /// Load document by name (for native, just calls load_document).
    pub fn load_document_by_name(_name: &str) {
        load_document()
    }

    /// Export PNG to file using native file dialog (async, non-blocking).
    pub fn export_png(png_data: &[u8], name: &str) {
        let data = png_data.to_vec();
        let default_name = format!("{}.png", name);
        std::thread::spawn(move || {
            let dialog = rfd::FileDialog::new()
                .set_title("Export PNG")
                .set_file_name(&default_name)
                .add_filter("PNG Image", &["png"]);

            if let Some(path) = dialog.save_file() {
                if let Err(e) = std::fs::write(&path, &data) {
                    log::error!("Failed to write PNG: {}", e);
                } else {
                    log::info!("Exported PNG to: {:?}", path);
                }
            }
        });
    }

    /// Copy PNG to clipboard.
    pub fn copy_png_to_clipboard(png_data: &[u8], width: u32, height: u32) {
        // arboard expects RGBA pixel data, not PNG encoded data
        // We need to decode if it's PNG, or pass raw RGBA
        let image_data = arboard::ImageData {
            width: width as usize,
            height: height as usize,
            bytes: std::borrow::Cow::Borrowed(png_data),
        };

        match arboard::Clipboard::new() {
            Ok(mut clipboard) => {
                if let Err(e) = clipboard.set_image(image_data) {
                    log::error!("Failed to copy PNG to clipboard: {}", e);
                } else {
                    log::info!("PNG copied to clipboard ({}x{})", width, height);
                }
            }
            Err(e) => log::error!("Failed to access clipboard: {}", e),
        }
    }

    /// Paste image from clipboard as a new Image shape.
    /// Returns None if no image is in the clipboard.
    pub fn paste_image_from_clipboard(
        canvas: &drafftink_core::canvas::Canvas,
    ) -> Option<drafftink_core::shapes::Shape> {
        use drafftink_core::shapes::{Image, ImageFormat, Shape};
        use kurbo::Point;

        match arboard::Clipboard::new() {
            Ok(mut clipboard) => {
                match clipboard.get_image() {
                    Ok(img_data) => {
                        // img_data contains RGBA pixel data
                        let width = img_data.width as u32;
                        let height = img_data.height as u32;

                        // Encode as PNG for storage (more compact than raw RGBA)
                        let mut png_data = Vec::new();
                        {
                            let mut encoder = ::png::Encoder::new(&mut png_data, width, height);
                            encoder.set_color(::png::ColorType::Rgba);
                            encoder.set_depth(::png::BitDepth::Eight);
                            if let Ok(mut writer) = encoder.write_header() {
                                if writer.write_image_data(&img_data.bytes).is_err() {
                                    log::error!("Failed to encode clipboard image as PNG");
                                    return None;
                                }
                            }
                        }

                        // Position at viewport center
                        let viewport_center = canvas.camera.screen_to_world(Point::new(
                            canvas.viewport_size.width / 2.0,
                            canvas.viewport_size.height / 2.0,
                        ));
                        let position = Point::new(
                            viewport_center.x - width as f64 / 2.0,
                            viewport_center.y - height as f64 / 2.0,
                        );

                        // Create image shape, scaled to fit if too large
                        let max_size = 800.0;
                        let mut image =
                            Image::new(position, &png_data, width, height, ImageFormat::Png);
                        if width as f64 > max_size || height as f64 > max_size {
                            image = image.fit_within(max_size, max_size);
                            // Re-center after fitting
                            image.position = Point::new(
                                viewport_center.x - image.width / 2.0,
                                viewport_center.y - image.height / 2.0,
                            );
                        }

                        log::info!("Pasted image from clipboard: {}x{}", width, height);
                        Some(Shape::Image(image))
                    }
                    Err(_) => {
                        // No image in clipboard
                        None
                    }
                }
            }
            Err(e) => {
                log::error!("Failed to access clipboard: {}", e);
                None
            }
        }
    }
}

#[cfg(target_arch = "wasm32")]
pub mod file_ops {
    use crate::settings::UserSettings;
    use drafftink_core::canvas::CanvasDocument;
    use drafftink_core::storage::{IndexedDbStorage, Storage};
    use std::cell::RefCell;
    use std::rc::Rc;
    use wasm_bindgen::prelude::*;

    thread_local! {
        static STORAGE: Rc<IndexedDbStorage> = Rc::new(IndexedDbStorage::new());
        static PENDING_DOCUMENT: RefCell<Option<CanvasDocument>> = const { RefCell::new(None) };
        static PENDING_DOCUMENT_LIST: RefCell<Option<Vec<String>>> = const { RefCell::new(None) };
        static PENDING_CLIPBOARD_TEXT: RefCell<Option<String>> = const { RefCell::new(None) };
        static PENDING_MATH_CLIPBOARD: RefCell<Option<String>> = const { RefCell::new(None) };
        static PENDING_LIBRARY: RefCell<Option<(String, CanvasDocument)>> = const { RefCell::new(None) };
        static PENDING_INTRO_JSON: RefCell<Option<(String, String)>> = const { RefCell::new(None) };
        static PENDING_EXPORT_FOLDER: RefCell<Option<String>> = const { RefCell::new(None) };
        static PENDING_LOCAL_FONTS: RefCell<Option<Vec<(String, String)>>> = const { RefCell::new(None) };
        static PENDING_LOCAL_FONT_BYTES: RefCell<std::collections::VecDeque<(String, String, Vec<u8>)>> = const { RefCell::new(std::collections::VecDeque::new()) };
        static PENDING_FONT_ERROR: RefCell<Option<String>> = const { RefCell::new(None) };
    }

    /// Ask Chrome/Edge for the local fonts installed on this computer.
    pub fn query_local_fonts_async() {
        wasm_bindgen_futures::spawn_local(async {
            let Some(window) = web_sys::window() else {
                return;
            };
            let Ok(value) = js_sys::Reflect::get(
                window.as_ref(),
                &JsValue::from_str("drafftinkQueryLocalFonts"),
            ) else {
                return;
            };
            let Ok(function) = value.dyn_into::<js_sys::Function>() else {
                return;
            };
            let Ok(result) = function.call0(window.as_ref()) else {
                return;
            };
            let Ok(promise) = result.dyn_into::<js_sys::Promise>() else {
                return;
            };
            let Ok(value) = wasm_bindgen_futures::JsFuture::from(promise).await else {
                return;
            };

            let array = js_sys::Array::from(&value);
            let mut fonts = Vec::new();
            for entry in array.iter() {
                let family = js_sys::Reflect::get(&entry, &JsValue::from_str("family"))
                    .ok()
                    .and_then(|v| v.as_string())
                    .unwrap_or_default();
                let postscript = js_sys::Reflect::get(&entry, &JsValue::from_str("postscriptName"))
                    .ok()
                    .and_then(|v| v.as_string())
                    .unwrap_or_default();
                if !family.is_empty() && !postscript.is_empty() {
                    fonts.push((family, postscript));
                }
            }
            fonts.sort_by(|a, b| a.0.to_lowercase().cmp(&b.0.to_lowercase()));
            fonts.dedup();

            PENDING_LOCAL_FONTS.with(|cell| {
                *cell.borrow_mut() = Some(fonts);
            });
            schedule_repaint(0);
        });
    }

    pub fn take_pending_local_fonts() -> Option<Vec<(String, String)>> {
        PENDING_LOCAL_FONTS.with(|cell| cell.borrow_mut().take())
    }

    pub fn schedule_repaint(delay_ms: i32) {
        if let Some(window) = web_sys::window() {
            if let Ok(value) = js_sys::Reflect::get(
                window.as_ref(),
                &JsValue::from_str("drafftinkRequestRepaint"),
            ) {
                if let Ok(function) = value.dyn_into::<js_sys::Function>() {
                    let _ = function.call1(window.as_ref(), &JsValue::from_f64(delay_ms as f64));
                }
            }
        }
    }
    pub fn load_local_font_async(family: String, postscript: String) {
        load_font(family, postscript, true);
    }
    pub fn restore_local_font_async(family: String, postscript: String) {
        load_font(family, postscript, false);
    }
    fn load_font(family: String, postscript: String, allow_prompt: bool) {
        wasm_bindgen_futures::spawn_local(async move {
            let result = async {
                let window = web_sys::window().ok_or(())?;
                let function = js_sys::Reflect::get(
                    window.as_ref(),
                    &JsValue::from_str("drafftinkReadLocalFont"),
                )
                .map_err(|_| ())?
                .dyn_into::<js_sys::Function>()
                .map_err(|_| ())?;
                let promise = function
                    .call2(
                        window.as_ref(),
                        &JsValue::from_str(&postscript),
                        &JsValue::from_bool(allow_prompt),
                    )
                    .map_err(|_| ())?
                    .dyn_into::<js_sys::Promise>()
                    .map_err(|_| ())?;
                let value = wasm_bindgen_futures::JsFuture::from(promise)
                    .await
                    .map_err(|_| ())?;
                let bytes =
                    js_sys::Reflect::get(&value, &JsValue::from_str("bytes")).map_err(|_| ())?;
                let bytes = js_sys::Uint8Array::new(&bytes).to_vec();
                if bytes.is_empty() {
                    return Err(());
                }
                Ok(bytes)
            }
            .await;
            match result {
                Ok(bytes) => PENDING_LOCAL_FONT_BYTES.with(|cell| cell.borrow_mut().push_back((family,postscript,bytes))),
                Err(_) => PENDING_FONT_ERROR.with(|cell| *cell.borrow_mut()=Some("Police introuvable : vérifiez son installation et actualisez la liste dans Settings.".into())),
            }
            schedule_repaint(0);
        });
    }
    pub fn take_pending_local_font_bytes() -> Option<(String, String, Vec<u8>)> {
        PENDING_LOCAL_FONT_BYTES.with(|cell| cell.borrow_mut().pop_front())
    }
    pub fn take_font_error() -> Option<String> {
        PENDING_FONT_ERROR.with(|cell| cell.borrow_mut().take())
    }

    /// Request clipboard text read (async). Result will be available via take_pending_clipboard_text().
    pub fn request_clipboard_text() {
        wasm_bindgen_futures::spawn_local(async {
            if let Some(text) = read_clipboard_text_async().await {
                PENDING_CLIPBOARD_TEXT.with(|cell| {
                    *cell.borrow_mut() = Some(text);
                });
            }
        });
    }

    /// Request clipboard text for math editor (async).
    pub fn request_clipboard_text_for_math() {
        wasm_bindgen_futures::spawn_local(async {
            if let Some(text) = read_clipboard_text_async().await {
                PENDING_MATH_CLIPBOARD.with(|cell| {
                    *cell.borrow_mut() = Some(text);
                });
            }
        });
    }

    /// Take pending math clipboard text if available.
    pub fn take_pending_math_clipboard() -> Option<String> {
        PENDING_MATH_CLIPBOARD.with(|cell| cell.borrow_mut().take())
    }

    /// Take pending clipboard text if available.
    pub fn take_pending_clipboard_text() -> Option<String> {
        PENDING_CLIPBOARD_TEXT.with(|cell| cell.borrow_mut().take())
    }

    async fn read_clipboard_text_async() -> Option<String> {
        let window = web_sys::window()?;
        let clipboard = window.navigator().clipboard();
        let promise = clipboard.read_text();
        wasm_bindgen_futures::JsFuture::from(promise)
            .await
            .ok()?
            .as_string()
    }

    /// Copy text to clipboard (fire and forget).
    pub fn copy_text_to_clipboard(text: &str) {
        let text = text.to_string();
        wasm_bindgen_futures::spawn_local(async move {
            if let Some(window) = web_sys::window() {
                let clipboard = window.navigator().clipboard();
                let promise = clipboard.write_text(&text);
                let _ = wasm_bindgen_futures::JsFuture::from(promise).await;
            }
        });
    }

    /// Save document to IndexedDB (real persistence).
    pub fn save_document(document: &CanvasDocument, name: &str) {
        let doc_id = name.to_string();
        let doc_clone = document.persisted_copy();

        STORAGE.with(|storage| {
            let storage = storage.clone();
            wasm_bindgen_futures::spawn_local(async move {
                let future = storage.save(&doc_id, &doc_clone);
                match future.await {
                    Ok(()) => {
                        log::info!("Document saved to IndexedDB: {}", doc_id);
                        // Also save as "last document" for auto-restore
                        let last_future = storage.save("__last__", &doc_clone);
                        if let Err(e) = last_future.await {
                            log::warn!("Failed to save as last document: {:?}", e);
                        }
                    }
                    Err(e) => log::error!("Failed to save document: {:?}", e),
                }
            });
        });
    }

    /// Auto-save document to IndexedDB (silent, no logging).
    pub fn autosave_document(document: &CanvasDocument) {
        let doc_clone = document.persisted_copy();

        STORAGE.with(|storage| {
            let storage = storage.clone();
            wasm_bindgen_futures::spawn_local(async move {
                let future = storage.save("__last__", &doc_clone);
                if let Err(e) = future.await {
                    log::warn!("Autosave failed: {:?}", e);
                }
            });
        });
    }

    /// Load document from IndexedDB - triggers async load.
    /// Use `take_pending_document()` to retrieve the loaded document.
    pub fn load_document_async() {
        STORAGE.with(|storage| {
            let storage = storage.clone();
            wasm_bindgen_futures::spawn_local(async move {
                // Try to load the last saved document
                let future = storage.load("__last__");
                match future.await {
                    Ok(doc) => {
                        log::info!("Document loaded from IndexedDB: {}", doc.name);
                        set_pending_document(doc);
                    }
                    Err(_) => {
                        // No saved document - load intro as default
                        static INTRO_JSON: &str = include_str!("../assets/intro.json");
                        if let Ok(doc) =
                            drafftink_core::canvas::CanvasDocument::from_json(INTRO_JSON)
                        {
                            log::info!("Loading intro document as default");
                            set_pending_document(doc);
                        }
                    }
                }
            });
        });
    }

    /// Restore the last autosaved document, or the configured intro document.
    /// An empty intro setting deliberately leaves a blank canvas.
    pub fn try_load_last_document(settings: &UserSettings) {
        let settings = settings.clone();
        STORAGE.with(|storage| {
            let storage = storage.clone();
            wasm_bindgen_futures::spawn_local(async move {
                if settings.restore_last_document {
                    if let Ok(doc) = storage.load("__last__").await {
                        log::info!("Restored last autosaved document: {}", doc.name);
                        set_pending_document(doc);
                        return;
                    }
                }

                if !settings.intro_json.trim().is_empty() {
                    match CanvasDocument::from_json(&settings.intro_json) {
                        Ok(doc) => {
                            log::info!("Loading configured intro document");
                            set_pending_document(doc);
                        }
                        Err(e) => log::warn!("Configured intro document is invalid: {}", e),
                    }
                }
            });
        });
    }

    /// Load document by name from IndexedDB - triggers async load.
    pub fn load_document_by_name_async(name: &str) {
        let doc_name = name.to_string();
        STORAGE.with(|storage| {
            let storage = storage.clone();
            wasm_bindgen_futures::spawn_local(async move {
                let future = storage.load(&doc_name);
                match future.await {
                    Ok(doc) => {
                        log::info!("Document loaded from IndexedDB: {}", doc.name);
                        set_pending_document(doc);
                    }
                    Err(e) => {
                        log::error!("Failed to load document '{}': {:?}", doc_name, e);
                    }
                }
            });
        });
    }

    /// List all documents in IndexedDB - triggers async list.
    pub fn list_documents_async() {
        STORAGE.with(|storage| {
            let storage = storage.clone();
            wasm_bindgen_futures::spawn_local(async move {
                let future = storage.list();
                match future.await {
                    Ok(docs) => {
                        let filtered: Vec<String> =
                            docs.into_iter().filter(|id| id != "__last__").collect();
                        PENDING_DOCUMENT_LIST.with(|list| {
                            *list.borrow_mut() = Some(filtered);
                        });
                    }
                    Err(e) => {
                        log::error!("Failed to list documents: {:?}", e);
                    }
                }
            });
        });
    }

    /// Take the pending document list.
    pub fn take_pending_document_list() -> Option<Vec<String>> {
        PENDING_DOCUMENT_LIST.with(|list| list.borrow_mut().take())
    }

    /// Download document as JSON file (export).
    pub fn download_document(document: &CanvasDocument, name: &str) {
        match document.to_json() {
            Ok(json) => download_file(&format!("{}.json", name), &json, "application/json"),
            Err(e) => log::error!("Failed to serialize document: {}", e),
        }
    }

    /// Upload document from file input - triggers async file picker.
    /// Use `take_pending_document()` to retrieve the loaded document.
    pub fn upload_document_async() {
        trigger_file_input_async();
    }

    /// Pick a JSON document to use as the startup intro.
    pub fn import_intro_json_async() {
        wasm_bindgen_futures::spawn_local(async {
            if let Err(e) = import_intro_json_impl().await {
                log::warn!("Failed to import intro JSON: {:?}", e);
            }
        });
    }

    pub fn take_pending_intro_json() -> Option<(String, String)> {
        PENDING_INTRO_JSON.with(|cell| cell.borrow_mut().take())
    }

    async fn import_intro_json_impl() -> Result<(), JsValue> {
        let window = web_sys::window().ok_or("No window")?;
        let document = window.document().ok_or("No document")?;
        let input: web_sys::HtmlInputElement = document.create_element("input")?.dyn_into()?;
        input.set_type("file");
        input.set_accept(".json");
        input.style().set_property("display", "none").ok();
        document.body().ok_or("No body")?.append_child(&input)?;

        let file = wait_for_file_selection(&input).await;
        input.remove();
        let file = file?;
        let name = file.name();
        let text = wasm_bindgen_futures::JsFuture::from(file.text())
            .await?
            .as_string()
            .ok_or("Failed to read intro JSON")?;

        CanvasDocument::from_json(&text)
            .map_err(|e| JsValue::from_str(&format!("Invalid DrafftInk JSON: {}", e)))?;

        PENDING_INTRO_JSON.with(|cell| *cell.borrow_mut() = Some((name, text)));
        Ok(())
    }

    /// Consume a precision-touchpad pinch captured at the DOM level.
    /// Returns (zoom_factor, client_x, client_y).
    pub fn take_browser_pinch() -> Option<(f64, f64, f64)> {
        let window = web_sys::window()?;
        let key = JsValue::from_str("__drafftinkPinchGesture");
        let value = js_sys::Reflect::get(window.as_ref(), &key).ok()?;
        if value.is_null() || value.is_undefined() {
            return None;
        }
        let _ = js_sys::Reflect::set(window.as_ref(), &key, &JsValue::NULL);
        let factor = js_sys::Reflect::get(&value, &JsValue::from_str("factor"))
            .ok()?
            .as_f64()?;
        let x = js_sys::Reflect::get(&value, &JsValue::from_str("x"))
            .ok()?
            .as_f64()?;
        let y = js_sys::Reflect::get(&value, &JsValue::from_str("y"))
            .ok()?
            .as_f64()?;
        Some((factor, x, y))
    }

    /// Open Chrome/Edge's directory picker for the default export folder.
    pub fn choose_export_directory() {
        wasm_bindgen_futures::spawn_local(async {
            let Some(window) = web_sys::window() else {
                return;
            };
            let Ok(value) = js_sys::Reflect::get(
                window.as_ref(),
                &JsValue::from_str("drafftinkPickExportDirectory"),
            ) else {
                return;
            };
            let Ok(function) = value.dyn_into::<js_sys::Function>() else {
                return;
            };
            let Ok(result) = function.call0(window.as_ref()) else {
                return;
            };
            let Ok(promise) = result.dyn_into::<js_sys::Promise>() else {
                return;
            };
            if let Ok(value) = wasm_bindgen_futures::JsFuture::from(promise).await {
                if let Some(name) = value.as_string() {
                    if !name.is_empty() {
                        PENDING_EXPORT_FOLDER.with(|cell| *cell.borrow_mut() = Some(name));
                    }
                }
            }
        });
    }

    pub fn take_pending_export_folder() -> Option<String> {
        PENDING_EXPORT_FOLDER.with(|cell| cell.borrow_mut().take())
    }

    /// Take the pending document loaded from async operations.
    /// Returns Some if a document was loaded, None otherwise.
    pub fn take_pending_document() -> Option<CanvasDocument> {
        PENDING_DOCUMENT.with(|cell| cell.borrow_mut().take())
    }

    fn set_pending_document(doc: CanvasDocument) {
        PENDING_DOCUMENT.with(|cell| {
            *cell.borrow_mut() = Some(doc);
        });
    }

    /// Export PNG (triggers browser download).
    pub fn export_png(png_data: &[u8], name: &str) {
        download_binary_file(&format!("{}.png", name), png_data, "image/png");
    }

    pub fn browser_flag(name: &str) -> bool {
        web_sys::window()
            .and_then(|w| js_sys::Reflect::get(&w, &name.into()).ok())
            .and_then(|v| v.as_bool())
            .unwrap_or(false)
    }
    pub fn save_status() -> String {
        web_sys::window()
            .and_then(|w| js_sys::Reflect::get(&w, &"drafftinkSaveStatus".into()).ok())
            .and_then(|v| v.as_string())
            .unwrap_or_default()
    }
    pub fn snapshot_filename(name: &str, extension: &str) -> String {
        web_sys::window()
            .and_then(|window| {
                js_sys::Reflect::get(&window, &"drafftinkSnapshotFilename".into())
                    .ok()
                    .and_then(|f| f.dyn_into::<js_sys::Function>().ok())
                    .and_then(|f| f.call2(&window, &name.into(), &extension.into()).ok())
                    .and_then(|v| v.as_string())
            })
            .unwrap_or_else(|| format!("{}.{}", name, extension))
    }
    pub fn export_png_automatic(bytes: &[u8], name: &str, json: Option<&str>) {
        let data = js_sys::Uint8Array::from(bytes);
        let parts = js_sys::Array::new();
        parts.push(&data);
        let options = web_sys::BlobPropertyBag::new();
        options.set_type("image/png");
        let Ok(blob) = web_sys::Blob::new_with_u8_array_sequence_and_options(&parts, &options)
        else {
            return;
        };
        let name = name.to_string();
        let json = json.unwrap_or_default().to_string();
        wasm_bindgen_futures::spawn_local(async move {
            let Some(window) = web_sys::window() else {
                return;
            };
            if let Ok(value) = js_sys::Reflect::get(&window, &"drafftinkSaveSnapshotPair".into()) {
                if let Ok(function) = value.dyn_into::<js_sys::Function>() {
                    let args = js_sys::Array::new();
                    args.push(&name.into());
                    args.push(&blob);
                    args.push(&JsValue::from_str(&json));
                    if let Ok(promise) = function
                        .apply(&window, &args)
                        .and_then(|v| v.dyn_into::<js_sys::Promise>())
                    {
                        let _ = wasm_bindgen_futures::JsFuture::from(promise).await;
                    }
                }
            }
        });
    }

    /// Copy PNG to clipboard using the async Clipboard API.
    /// This spawns an async task since clipboard.write() returns a Promise.
    pub fn copy_png_to_clipboard(png_data: Vec<u8>) {
        wasm_bindgen_futures::spawn_local(async move {
            if let Err(e) = copy_png_to_clipboard_async(png_data).await {
                log::error!("Failed to copy to clipboard: {:?}", e);
                // Could fall back to download here if needed
            }
        });
    }

    async fn copy_png_to_clipboard_async(png_data: Vec<u8>) -> Result<(), JsValue> {
        let window = web_sys::window().ok_or("No window")?;
        let navigator = window.navigator();
        let clipboard = navigator.clipboard();

        // Create a Blob from the PNG data
        let uint8_array = js_sys::Uint8Array::from(png_data.as_slice());
        let blob_parts = js_sys::Array::new();
        blob_parts.push(&uint8_array);

        let options = web_sys::BlobPropertyBag::new();
        options.set_type("image/png");

        let blob = web_sys::Blob::new_with_u8_array_sequence_and_options(&blob_parts, &options)?;

        // Create ClipboardItem with the PNG blob
        let item_options = js_sys::Object::new();
        js_sys::Reflect::set(&item_options, &"image/png".into(), &blob)?;

        let clipboard_item = web_sys::ClipboardItem::new_with_record_from_str_to_blob_promise(
            &item_options.unchecked_into(),
        )?;

        let items = js_sys::Array::new();
        items.push(&clipboard_item);

        // Write to clipboard (returns a Promise)
        let promise = clipboard.write(&items);
        wasm_bindgen_futures::JsFuture::from(promise).await?;

        log::info!("PNG copied to clipboard ({} bytes)", png_data.len());
        Ok(())
    }

    fn download_file(filename: &str, content: &str, mime_type: &str) {
        let blob_parts = js_sys::Array::new();
        blob_parts.push(&JsValue::from_str(content));
        let options = web_sys::BlobPropertyBag::new();
        options.set_type(mime_type);
        if let Ok(blob) = web_sys::Blob::new_with_str_sequence_and_options(&blob_parts, &options) {
            export_blob_or_download(filename.to_string(), blob);
        }
    }

    fn download_binary_file(filename: &str, data: &[u8], mime_type: &str) {
        let uint8_array = js_sys::Uint8Array::from(data);
        let blob_parts = js_sys::Array::new();
        blob_parts.push(&uint8_array);
        let options = web_sys::BlobPropertyBag::new();
        options.set_type(mime_type);
        if let Ok(blob) =
            web_sys::Blob::new_with_u8_array_sequence_and_options(&blob_parts, &options)
        {
            export_blob_or_download(filename.to_string(), blob);
        }
    }

    fn export_blob_or_download(filename: String, blob: web_sys::Blob) {
        wasm_bindgen_futures::spawn_local(async move {
            if try_save_blob_to_export_folder(&filename, &blob)
                .await
                .unwrap_or(false)
            {
                log::info!("Exported '{}' to configured folder", filename);
                return;
            }
            fallback_download_blob(&filename, &blob);
        });
    }

    async fn try_save_blob_to_export_folder(
        filename: &str,
        blob: &web_sys::Blob,
    ) -> Result<bool, JsValue> {
        let window = web_sys::window().ok_or("No window")?;
        let value = js_sys::Reflect::get(
            window.as_ref(),
            &JsValue::from_str("drafftinkSaveBlobToExportDirectory"),
        )?;
        let function: js_sys::Function = match value.dyn_into() {
            Ok(f) => f,
            Err(_) => return Ok(false),
        };
        let result =
            function.call2(window.as_ref(), &JsValue::from_str(filename), blob.as_ref())?;
        let promise: js_sys::Promise = match result.dyn_into() {
            Ok(p) => p,
            Err(_) => return Ok(false),
        };
        let value = wasm_bindgen_futures::JsFuture::from(promise).await?;
        Ok(value.as_bool().unwrap_or(false))
    }

    fn fallback_download_blob(filename: &str, blob: &web_sys::Blob) {
        let Some(window) = web_sys::window() else {
            return;
        };
        let Some(document) = window.document() else {
            return;
        };
        let Ok(url) = web_sys::Url::create_object_url_with_blob(blob) else {
            return;
        };
        let Ok(element) = document.create_element("a") else {
            return;
        };
        let Ok(a) = element.dyn_into::<web_sys::HtmlAnchorElement>() else {
            return;
        };
        a.set_href(&url);
        a.set_download(filename);
        // Keep the Blob URL alive until Chromium has started reading it.
        // Immediate revocation can cancel the asynchronous download entirely.
        a.style().set_property("display", "none").ok();
        if let Some(body) = document.body() {
            body.append_child(&a).ok();
        }
        a.click();
        a.remove();
        let cleanup = wasm_bindgen::closure::Closure::once_into_js(move || {
            let _ = web_sys::Url::revoke_object_url(&url);
        });
        let _ = window
            .set_timeout_with_callback_and_timeout_and_arguments_0(cleanup.unchecked_ref(), 1000);
    }

    fn trigger_file_input_async() {
        wasm_bindgen_futures::spawn_local(async {
            if let Err(e) = trigger_file_input_impl().await {
                log::error!("Failed to load file: {:?}", e);
            }
        });
    }

    /// Load an Excalidraw library (`.excalidrawlib`) via a browser file picker
    /// (async). Retrieve via `take_pending_library()`.
    pub fn load_excalidrawlib_async() {
        wasm_bindgen_futures::spawn_local(async {
            if let Err(e) = load_excalidrawlib_impl().await {
                log::error!("Failed to load library: {:?}", e);
            }
        });
    }

    async fn load_excalidrawlib_impl() -> Result<(), JsValue> {
        let window = web_sys::window().ok_or("No window")?;
        let document = window.document().ok_or("No document")?;

        let input: web_sys::HtmlInputElement = document.create_element("input")?.dyn_into()?;
        input.set_type("file");
        input.set_accept(".excalidrawlib,.json");
        input.style().set_property("display", "none").ok();
        document.body().ok_or("No body")?.append_child(&input)?;

        let file = wait_for_file_selection(&input).await;
        input.remove();
        let file = file?;

        // Use the file name (without extension) as the tab label.
        let filename = file.name();
        let name = filename
            .rsplit('/')
            .next()
            .unwrap_or(&filename)
            .strip_suffix(".excalidrawlib")
            .map(str::to_string)
            .unwrap_or(filename);

        let text: String = wasm_bindgen_futures::JsFuture::from(file.text())
            .await?
            .as_string()
            .ok_or("Failed to read file as text")?;

        match super::build_library_document(&text, &name) {
            Some(result) => {
                PENDING_LIBRARY.with(|cell| *cell.borrow_mut() = Some(result));
                Ok(())
            }
            None => Err(JsValue::from_str("Not a valid Excalidraw library")),
        }
    }

    /// Take a pending library `(name, document)` from an async load.
    pub fn take_pending_library() -> Option<(String, CanvasDocument)> {
        PENDING_LIBRARY.with(|cell| cell.borrow_mut().take())
    }

    async fn trigger_file_input_impl() -> Result<(), JsValue> {
        let window = web_sys::window().ok_or("No window")?;
        let document = window.document().ok_or("No document")?;

        let input: web_sys::HtmlInputElement = document.create_element("input")?.dyn_into()?;
        input.set_type("file");
        input.set_accept(".json,.excalidraw");
        input.style().set_property("display", "none").ok();

        document.body().ok_or("No body")?.append_child(&input)?;

        // Wait for file selection via Promise
        let file = wait_for_file_selection(&input).await;
        input.remove();

        let file = file?;
        let filename = file.name();
        let is_excalidraw = filename.to_lowercase().ends_with(".excalidraw");

        // Read file content using File.text() which returns a Promise
        let text: String = wasm_bindgen_futures::JsFuture::from(file.text())
            .await?
            .as_string()
            .ok_or("Failed to read file as text")?;

        let doc_result = if is_excalidraw {
            CanvasDocument::from_excalidraw(&text).map_err(|e| e.to_string())
        } else {
            CanvasDocument::from_json(&text).map_err(|e| e.to_string())
        };

        match doc_result {
            Ok(doc) => {
                log::info!("Document loaded: {}", doc.name);
                set_pending_document(doc);
                Ok(())
            }
            Err(e) => Err(JsValue::from_str(&format!(
                "Failed to parse document: {}",
                e
            ))),
        }
    }

    async fn wait_for_file_selection(
        input: &web_sys::HtmlInputElement,
    ) -> Result<web_sys::File, JsValue> {
        use wasm_bindgen::closure::Closure;

        let input_clone = input.clone();
        let promise = js_sys::Promise::new(&mut |resolve, _reject| {
            let input_inner = input_clone.clone();
            let onchange = Closure::once(Box::new(move |_: web_sys::Event| {
                let file = input_inner.files().and_then(|f| f.get(0));
                resolve.call1(&JsValue::NULL, &file.into()).ok();
            }) as Box<dyn FnOnce(_)>);
            input_clone.set_onchange(Some(onchange.as_ref().unchecked_ref()));
            onchange.forget();
        });

        input.click();

        let result = wasm_bindgen_futures::JsFuture::from(promise).await?;
        result.dyn_into::<web_sys::File>()
    }

    // Thread-local storage for pending pasted image
    thread_local! {
        static PENDING_IMAGE: RefCell<Option<drafftink_core::shapes::Shape>> = const { RefCell::new(None) };
    }

    /// Try to paste an image from the clipboard (async).
    /// The result will be available via `take_pending_image()`.
    pub fn paste_image_from_clipboard_async(
        viewport_width: f64,
        viewport_height: f64,
        camera_offset_x: f64,
        camera_offset_y: f64,
        camera_zoom: f64,
    ) {
        wasm_bindgen_futures::spawn_local(async move {
            if let Err(e) = paste_image_async(
                viewport_width,
                viewport_height,
                camera_offset_x,
                camera_offset_y,
                camera_zoom,
            )
            .await
            {
                log::debug!("No image in clipboard or paste failed: {:?}", e);
            }
        });
    }

    async fn paste_image_async(
        viewport_width: f64,
        viewport_height: f64,
        camera_offset_x: f64,
        camera_offset_y: f64,
        camera_zoom: f64,
    ) -> Result<(), JsValue> {
        use drafftink_core::shapes::{Image, ImageFormat, Shape};
        use kurbo::Point;

        let window = web_sys::window().ok_or("No window")?;
        let navigator = window.navigator();
        let clipboard = navigator.clipboard();

        // Read clipboard items
        let promise = clipboard.read();
        let result = wasm_bindgen_futures::JsFuture::from(promise).await?;
        let items: js_sys::Array = result.dyn_into()?;

        for i in 0..items.length() {
            let item: web_sys::ClipboardItem = items.get(i).dyn_into()?;
            let types = item.types();

            // Check for image types
            for j in 0..types.length() {
                if let Some(mime_type) = types.get(j).as_string() {
                    if mime_type.starts_with("image/") {
                        // Get the blob for this type
                        let blob_promise = item.get_type(&mime_type);
                        let blob: web_sys::Blob =
                            wasm_bindgen_futures::JsFuture::from(blob_promise)
                                .await?
                                .dyn_into()?;

                        // Read blob as array buffer
                        let array_buffer_promise = blob.array_buffer();
                        let array_buffer =
                            wasm_bindgen_futures::JsFuture::from(array_buffer_promise).await?;
                        let uint8_array = js_sys::Uint8Array::new(&array_buffer);
                        let data = uint8_array.to_vec();

                        // Determine format from mime type
                        let format = match mime_type.as_str() {
                            "image/png" => ImageFormat::Png,
                            "image/jpeg" => ImageFormat::Jpeg,
                            "image/webp" => ImageFormat::WebP,
                            _ => ImageFormat::Png, // Default to PNG
                        };

                        // Decode image to get dimensions
                        // We need to use the browser's Image API or decode in Rust
                        // For simplicity, we'll try to decode using the image crate via a simple approach
                        // But image crate is not available in WASM by default, so we'll use a different method

                        // Create an HTML Image element to decode
                        let blob_url = web_sys::Url::create_object_url_with_blob(&blob)?;

                        let (width, height) = decode_image_dimensions(&blob_url).await?;

                        web_sys::Url::revoke_object_url(&blob_url)?;

                        // Calculate viewport center in world coordinates
                        let viewport_center_x =
                            (viewport_width / 2.0 - camera_offset_x) / camera_zoom;
                        let viewport_center_y =
                            (viewport_height / 2.0 - camera_offset_y) / camera_zoom;

                        let position = Point::new(
                            viewport_center_x - width as f64 / 2.0,
                            viewport_center_y - height as f64 / 2.0,
                        );

                        // Create image shape, scaled to fit if too large
                        let max_size = 800.0;
                        let mut image = Image::new(position, &data, width, height, format);
                        if width as f64 > max_size || height as f64 > max_size {
                            image = image.fit_within(max_size, max_size);
                            image.position = Point::new(
                                viewport_center_x - image.width / 2.0,
                                viewport_center_y - image.height / 2.0,
                            );
                        }

                        log::info!("Pasted image from clipboard: {}x{}", width, height);
                        set_pending_image(Shape::Image(image));
                        return Ok(());
                    }
                }
            }
        }

        Err("No image found in clipboard".into())
    }

    async fn decode_image_dimensions(blob_url: &str) -> Result<(u32, u32), JsValue> {
        let window = web_sys::window().ok_or("No window")?;
        let document = window.document().ok_or("No document")?;
        let img: web_sys::HtmlImageElement = document.create_element("img")?.dyn_into()?;

        let promise = wait_for_image_load(&img);
        img.set_src(blob_url);
        promise.await?;

        Ok((img.natural_width(), img.natural_height()))
    }

    async fn wait_for_image_load(img: &web_sys::HtmlImageElement) -> Result<(), JsValue> {
        use wasm_bindgen::closure::Closure;

        let img_clone = img.clone();
        let promise = js_sys::Promise::new(&mut |resolve, reject| {
            let onload = Closure::once(Box::new(move |_: web_sys::Event| {
                resolve.call0(&JsValue::NULL).ok();
            }) as Box<dyn FnOnce(_)>);
            let onerror = Closure::once(Box::new(move |_: web_sys::Event| {
                reject
                    .call1(&JsValue::NULL, &"Failed to load image".into())
                    .ok();
            }) as Box<dyn FnOnce(_)>);
            img_clone.set_onload(Some(onload.as_ref().unchecked_ref()));
            img_clone.set_onerror(Some(onerror.as_ref().unchecked_ref()));
            onload.forget();
            onerror.forget();
        });

        wasm_bindgen_futures::JsFuture::from(promise).await?;
        Ok(())
    }

    /// Yield to the browser event loop via requestAnimationFrame.
    /// This allows WebGPU callbacks and other browser tasks to run.
    pub async fn yield_to_browser() {
        use wasm_bindgen::closure::Closure;

        let promise = js_sys::Promise::new(&mut |resolve, _reject| {
            let window = web_sys::window().expect("no window");
            let closure = Closure::once_into_js(move || {
                let _ = resolve.call0(&JsValue::NULL);
            });
            let _ = window.request_animation_frame(closure.unchecked_ref());
        });
        let _ = wasm_bindgen_futures::JsFuture::from(promise).await;
    }

    fn set_pending_image(shape: drafftink_core::shapes::Shape) {
        PENDING_IMAGE.with(|cell| {
            *cell.borrow_mut() = Some(shape);
        });
    }

    /// Take the pending image from clipboard paste.
    pub fn take_pending_image() -> Option<drafftink_core::shapes::Shape> {
        PENDING_IMAGE.with(|cell| cell.borrow_mut().take())
    }

    // Thread-local storage for pending pasted Excalidraw shapes
    thread_local! {
        static PENDING_EXCALIDRAW_SHAPES: RefCell<Option<(Vec<drafftink_core::shapes::Shape>, kurbo::Point)>> = const { RefCell::new(None) };
    }

    /// Try to import shapes from clipboard text: Excalidraw scene JSON first,
    /// then Mermaid diagram source. The result will be available via
    /// `take_pending_excalidraw_shapes()`.
    pub fn paste_shapes_from_clipboard_async(cursor_world: kurbo::Point) {
        wasm_bindgen_futures::spawn_local(async move {
            if let Some(text) = read_clipboard_text_async().await {
                let shapes =
                    drafftink_core::canvas::CanvasDocument::shapes_from_excalidraw_clipboard(&text)
                        .or_else(|| drafftink_core::shapes_from_mermaid(&text));
                if let Some(shapes) = shapes {
                    PENDING_EXCALIDRAW_SHAPES.with(|cell| {
                        *cell.borrow_mut() = Some((shapes, cursor_world));
                    });
                }
            }
        });
    }

    /// Take pending Excalidraw shapes from clipboard paste.
    pub fn take_pending_excalidraw_shapes()
    -> Option<(Vec<drafftink_core::shapes::Shape>, kurbo::Point)> {
        PENDING_EXCALIDRAW_SHAPES.with(|cell| cell.borrow_mut().take())
    }

    // Thread-local storage for pending dropped images
    thread_local! {
        static PENDING_DROPPED_IMAGES: RefCell<Vec<drafftink_core::shapes::Shape>> = const { RefCell::new(Vec::new()) };
    }

    /// Setup drag-drop handlers on the canvas element.
    /// Call this once after the canvas is created.
    pub fn setup_drag_drop_handlers(
        viewport_width: f64,
        viewport_height: f64,
        camera_offset_x: f64,
        camera_offset_y: f64,
        camera_zoom: f64,
    ) {
        use wasm_bindgen::closure::Closure;

        let window = web_sys::window().expect("No window");
        let document = window.document().expect("No document");

        // Find the canvas element
        if let Some(canvas) = document.query_selector("canvas").ok().flatten() {
            // Prevent default drag behavior
            let ondragover = Closure::wrap(Box::new(move |event: web_sys::DragEvent| {
                event.prevent_default();
            }) as Box<dyn Fn(_)>);

            canvas
                .add_event_listener_with_callback("dragover", ondragover.as_ref().unchecked_ref())
                .ok();
            ondragover.forget();

            // Handle drop
            let vw = viewport_width;
            let vh = viewport_height;
            let cox = camera_offset_x;
            let coy = camera_offset_y;
            let cz = camera_zoom;

            let ondrop = Closure::wrap(Box::new(move |event: web_sys::DragEvent| {
                event.prevent_default();

                if let Some(data_transfer) = event.data_transfer() {
                    if let Some(files) = data_transfer.files() {
                        for i in 0..files.length() {
                            if let Some(file) = files.get(i) {
                                let file_name = file.name();
                                let file_type = file.type_();

                                // Check if it's an image
                                if file_type.starts_with("image/") {
                                    handle_dropped_file(file, vw, vh, cox, coy, cz);
                                } else if let Some(ext) = file_name.split('.').last() {
                                    if matches!(
                                        ext.to_lowercase().as_str(),
                                        "png" | "jpg" | "jpeg" | "webp"
                                    ) {
                                        handle_dropped_file(file, vw, vh, cox, coy, cz);
                                    }
                                }
                            }
                        }
                    }
                }
            }) as Box<dyn Fn(_)>);

            canvas
                .add_event_listener_with_callback("drop", ondrop.as_ref().unchecked_ref())
                .ok();
            ondrop.forget();
        }
    }

    fn handle_dropped_file(
        file: web_sys::File,
        viewport_width: f64,
        viewport_height: f64,
        camera_offset_x: f64,
        camera_offset_y: f64,
        camera_zoom: f64,
    ) {
        wasm_bindgen_futures::spawn_local(async move {
            if let Err(e) = process_dropped_file(
                file,
                viewport_width,
                viewport_height,
                camera_offset_x,
                camera_offset_y,
                camera_zoom,
            )
            .await
            {
                log::error!("Failed to process dropped file: {:?}", e);
            }
        });
    }

    async fn process_dropped_file(
        file: web_sys::File,
        viewport_width: f64,
        viewport_height: f64,
        camera_offset_x: f64,
        camera_offset_y: f64,
        camera_zoom: f64,
    ) -> Result<(), JsValue> {
        use drafftink_core::shapes::{Image, ImageFormat, Shape};
        use kurbo::Point;

        let file_name = file.name();
        let file_type = file.type_();

        // Read file as array buffer
        let array_buffer_promise = file.array_buffer();
        let array_buffer = wasm_bindgen_futures::JsFuture::from(array_buffer_promise).await?;
        let uint8_array = js_sys::Uint8Array::new(&array_buffer);
        let data = uint8_array.to_vec();

        // Check for embedded scene data in PNG files
        if file_type == "image/png" || file_name.ends_with(".png") {
            if let Some(json) = super::extract_scene_from_png(&data) {
                log::info!("Found embedded scene in PNG, loading as document");
                add_pending_dropped_document(json);
                return Ok(());
            }
        }

        // Determine format
        let format = if file_type == "image/jpeg"
            || file_name.ends_with(".jpg")
            || file_name.ends_with(".jpeg")
        {
            ImageFormat::Jpeg
        } else if file_type == "image/webp" || file_name.ends_with(".webp") {
            ImageFormat::WebP
        } else {
            ImageFormat::Png
        };

        // Create blob URL to decode dimensions
        let blob: web_sys::Blob = file.into();
        let blob_url = web_sys::Url::create_object_url_with_blob(&blob)?;
        let (width, height) = decode_image_dimensions(&blob_url).await?;
        web_sys::Url::revoke_object_url(&blob_url)?;

        // Calculate viewport center in world coordinates
        let viewport_center_x = (viewport_width / 2.0 - camera_offset_x) / camera_zoom;
        let viewport_center_y = (viewport_height / 2.0 - camera_offset_y) / camera_zoom;

        let position = Point::new(
            viewport_center_x - width as f64 / 2.0,
            viewport_center_y - height as f64 / 2.0,
        );

        // Create image shape, scaled to fit if too large
        let max_size = 800.0;
        let mut image = Image::new(position, &data, width, height, format);
        if width as f64 > max_size || height as f64 > max_size {
            image = image.fit_within(max_size, max_size);
            image.position = Point::new(
                viewport_center_x - image.width / 2.0,
                viewport_center_y - image.height / 2.0,
            );
        }

        log::info!("Dropped image: {} ({}x{})", file_name, width, height);
        add_pending_dropped_image(Shape::Image(image));

        Ok(())
    }

    /// Queue a dropped document (from PNG with embedded scene) for loading.
    fn add_pending_dropped_document(json: String) {
        PENDING_DROPPED_DOCUMENT.with(|cell| {
            cell.borrow_mut().replace(json);
        });
    }

    thread_local! {
        static PENDING_DROPPED_DOCUMENT: std::cell::RefCell<Option<String>> = const { std::cell::RefCell::new(None) };
    }

    /// Take any pending dropped document JSON.
    pub fn take_pending_dropped_document() -> Option<String> {
        PENDING_DROPPED_DOCUMENT.with(|cell| cell.borrow_mut().take())
    }

    fn add_pending_dropped_image(shape: drafftink_core::shapes::Shape) {
        PENDING_DROPPED_IMAGES.with(|cell| {
            cell.borrow_mut().push(shape);
        });
    }

    /// Take all pending dropped images.
    pub fn take_pending_dropped_images() -> Vec<drafftink_core::shapes::Shape> {
        PENDING_DROPPED_IMAGES.with(|cell| std::mem::take(&mut *cell.borrow_mut()))
    }
}

/// Render a Vello scene to PNG bytes (native version - blocking).
#[cfg(not(target_arch = "wasm32"))]
fn render_scene_to_png(
    device: &vello::wgpu::Device,
    queue: &vello::wgpu::Queue,
    vello_renderer: &mut vello::Renderer,
    scene: &Scene,
    width: u32,
    height: u32,
) -> Option<PngRenderResult> {
    if width == 0 || height == 0 {
        return None;
    }

    // Create offscreen texture for rendering
    let texture = device.create_texture(&vello::wgpu::TextureDescriptor {
        label: Some("png export texture"),
        size: vello::wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: vello::wgpu::TextureDimension::D2,
        format: vello::wgpu::TextureFormat::Rgba8Unorm,
        usage: vello::wgpu::TextureUsages::STORAGE_BINDING
            | vello::wgpu::TextureUsages::COPY_SRC
            | vello::wgpu::TextureUsages::TEXTURE_BINDING,
        view_formats: &[],
    });

    let texture_view = texture.create_view(&vello::wgpu::TextureViewDescriptor::default());

    // Render the scene
    let params = RenderParams {
        base_color: Color::WHITE,
        width,
        height,
        antialiasing_method: AaConfig::Area,
    };

    if let Err(e) = vello_renderer.render_to_texture(device, queue, scene, &texture_view, &params) {
        log::error!("Failed to render scene for PNG export: {:?}", e);
        return None;
    }

    // Create buffer to read back pixels
    let bytes_per_row = (width * 4).next_multiple_of(256); // wgpu alignment requirement
    let buffer_size = (bytes_per_row * height) as u64;

    let readback_buffer = device.create_buffer(&vello::wgpu::BufferDescriptor {
        label: Some("png readback buffer"),
        size: buffer_size,
        usage: vello::wgpu::BufferUsages::COPY_DST | vello::wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });

    // Copy texture to buffer
    let mut encoder = device.create_command_encoder(&vello::wgpu::CommandEncoderDescriptor {
        label: Some("png copy encoder"),
    });

    encoder.copy_texture_to_buffer(
        vello::wgpu::TexelCopyTextureInfo {
            texture: &texture,
            mip_level: 0,
            origin: vello::wgpu::Origin3d::ZERO,
            aspect: vello::wgpu::TextureAspect::All,
        },
        vello::wgpu::TexelCopyBufferInfo {
            buffer: &readback_buffer,
            layout: vello::wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(bytes_per_row),
                rows_per_image: Some(height),
            },
        },
        vello::wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
    );

    queue.submit(std::iter::once(encoder.finish()));

    // Map buffer and read pixels
    let buffer_slice = readback_buffer.slice(..);
    let (tx, rx) = std::sync::mpsc::channel();
    buffer_slice.map_async(vello::wgpu::MapMode::Read, move |result| {
        tx.send(result).ok();
    });

    // Wait for GPU to finish (blocking - native only)
    let _ = device.poll(vello::wgpu::PollType::wait_indefinitely());

    if rx.recv().ok()?.is_err() {
        log::error!("Failed to map buffer for PNG readback");
        return None;
    }

    let data = buffer_slice.get_mapped_range();

    // Remove row padding if any
    let mut rgba_data = Vec::with_capacity((width * height * 4) as usize);
    for row in 0..height {
        let row_start = (row * bytes_per_row) as usize;
 