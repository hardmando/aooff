use crate::protocol::{App, Project, Request, Response};
use fuzzy_matcher::FuzzyMatcher;
use fuzzy_matcher::skim::SkimMatcherV2;
use rkyv::util::AlignedVec;
use std::io::{Read, Write};
use std::os::fd::AsFd;
use std::os::unix::net::UnixStream;
use std::process::Command;

use cosmic_text::{Attrs, Buffer, Color, FontSystem, Metrics, Shaping, SwashCache};
use layershellev::reexport::*;
use layershellev::*;
use winit::event::ElementState;
use winit::keyboard::{Key, NamedKey};

/// Item in the suggestion list — either an App or a Project.
#[derive(Clone)]
pub enum SuggestionItem {
    App(App),
    Project(Project),
}

impl SuggestionItem {
    pub fn name(&self) -> &str {
        match self {
            SuggestionItem::App(a) => &a.name,
            SuggestionItem::Project(p) => &p.name,
        }
    }

    pub fn tag(&self) -> &str {
        match self {
            SuggestionItem::App(_) => "App",
            SuggestionItem::Project(_) => "Project",
        }
    }

    pub fn execute(&self) {
        match self {
            SuggestionItem::App(a) => {
                let _ = Command::new(&*a.path).spawn();
            }
            SuggestionItem::Project(p) => {
                let path = p.path.to_string();
                let _ = Command::new("sh")
                    .arg("-c")
                    .arg(format!(
                        "kitty -e tmux new-session sh -c \"cd '{}' && tdl c\"",
                        path
                    ))
                    .spawn();
            }
        }
    }
}

pub struct PopupState {
    pub query: String,
    pub all_items: Vec<SuggestionItem>,
    pub filtered: Vec<SuggestionItem>,
    pub selected: usize,
    matcher: SkimMatcherV2,
}

impl PopupState {
    pub fn new(projects: Vec<Project>, apps: Vec<App>) -> Self {
        let mut all_items: Vec<SuggestionItem> = Vec::with_capacity(projects.len() + apps.len());

        for p in projects {
            all_items.push(SuggestionItem::Project(p));
        }
        for a in apps {
            all_items.push(SuggestionItem::App(a));
        }

        let filtered = all_items.clone();

        PopupState {
            query: String::new(),
            all_items,
            filtered,
            selected: 0,
            matcher: SkimMatcherV2::default(),
        }
    }

    pub fn update_filter(&mut self) {
        if self.query.is_empty() {
            self.filtered = self.all_items.clone();
        } else {
            let mut matches: Vec<(i64, SuggestionItem)> = self
                .all_items
                .iter()
                .filter_map(|item| {
                    self.matcher
                        .fuzzy_match(item.name(), &self.query)
                        .map(|score| (score, item.clone()))
                })
                .collect();
            matches.sort_by(|a, b| b.0.cmp(&a.0));
            self.filtered = matches.into_iter().map(|(_, item)| item).collect();
        }

        self.selected = 0;
    }
}

/// Connect to the daemon and fetch all projects and apps.
fn fetch_data() -> Result<(Vec<Project>, Vec<App>), String> {
    let mut stream =
        UnixStream::connect("/tmp/aooff.sock").map_err(|e| format!("Connect failed: {}", e))?;

    let request_bytes = rkyv::to_bytes::<rkyv::rancor::Error>(&Request::GetAll)
        .map_err(|e| format!("Serialize failed: {}", e))?;

    stream
        .write_all(&request_bytes)
        .map_err(|e| format!("Write failed: {}", e))?;

    stream
        .shutdown(std::net::Shutdown::Write)
        .map_err(|e| format!("Shutdown failed: {}", e))?;

    let mut buffer = Vec::new();
    stream
        .read_to_end(&mut buffer)
        .map_err(|e| format!("Read failed: {}", e))?;

    let mut aligned: AlignedVec<16> = AlignedVec::new();
    aligned.extend_from_slice(&buffer);

    let response = rkyv::from_bytes::<Response, rkyv::rancor::Error>(&aligned)
        .map_err(|e| format!("Deserialize failed: {}", e))?;

    match response {
        Response::All { projects, apps } => Ok((projects, apps)),
        Response::Projects(projects) => Ok((projects, Vec::new())),
        Response::Error(e) => Err(e),
    }
}

pub fn run() {
    let (projects, apps) = match fetch_data() {
        Ok(data) => data,
        Err(e) => {
            eprintln!("Failed to connect to daemon: {}", e);
            std::process::exit(1);
        }
    };

    let mut state = PopupState::new(projects, apps);

    let mut font_system = FontSystem::new();
    let mut swash_cache = SwashCache::new();

    // Top, Right, Bottom, Left margin
    let ev: WindowState<()> = WindowState::new("aooff")
        .with_allscreens()
        .with_size((600, 400))
        .with_layer(Layer::Top)
        .with_anchor(Anchor::Bottom | Anchor::Left)
        .with_margin((0, 0, 20, 20))
        .with_keyboard_interacivity(KeyboardInteractivity::Exclusive)
        .build()
        .unwrap();

    let mut width = 600;
    let mut height = 400;
    let mut draw_file: Option<std::fs::File> = None;

    ev.running(move |event, _ev, _index| match event {
        LayerShellEvent::InitRequest => ReturnData::RequestBind,
        LayerShellEvent::BindProvide(_globals, _qh) => ReturnData::RequestCompositor,
        LayerShellEvent::CompositorProvide(compositor, qh) => {
            for x in _ev.get_unit_iter() {
                let region = compositor.create_region(qh, ());
                region.add(0, 0, 600, 400);
                x.get_wlsurface().set_input_region(Some(&region));
            }
            ReturnData::None
        }
        LayerShellEvent::RequestBuffer(file, shm, qh, init_w, init_h) => {
            width = init_w;
            height = init_h;
            draw(
                file,
                width,
                height,
                &state,
                &mut font_system,
                &mut swash_cache,
            );
            draw_file = file.try_clone().ok();
            let pool = shm.create_pool(file.as_fd(), (width * height * 4) as i32, qh, ());
            ReturnData::WlBuffer(pool.create_buffer(
                0,
                width as i32,
                height as i32,
                (width * 4) as i32,
                wl_shm::Format::Argb8888,
                qh,
                (),
            ))
        }
        LayerShellEvent::RequestMessages(DispatchMessage::RequestRefresh {
            width: w,
            height: h,
            ..
        }) => {
            width = *w;
            height = *h;
            if let Some(file) = draw_file.as_mut() {
                draw(
                    file,
                    width,
                    height,
                    &state,
                    &mut font_system,
                    &mut swash_cache,
                );
                if let Some(idx) = _index {
                    if let Some(unit) = _ev.get_unit_with_id(idx) {
                        unit.refresh();
                    }
                }
            }
            ReturnData::None
        }
        LayerShellEvent::RequestMessages(DispatchMessage::KeyboardInput { event, .. }) => {
            if event.state == ElementState::Pressed {
                match &event.logical_key {
                    Key::Named(NamedKey::Escape) => {
                        return ReturnData::RequestExit;
                    }
                    Key::Named(NamedKey::Enter) => {
                        if let Some(item) = state.filtered.get(state.selected) {
                            item.execute();
                        }
                        return ReturnData::RequestExit;
                    }
                    Key::Named(NamedKey::Backspace) => {
                        if !state.query.is_empty() {
                            state.query.pop();
                            state.update_filter();
                            _ev.request_refresh_all(layershellev::RefreshRequest::NextFrame);
                            return ReturnData::None;
                        }
                    }
                    Key::Named(NamedKey::ArrowUp) => {
                        if state.selected > 0 {
                            state.selected -= 1;
                            _ev.request_refresh_all(layershellev::RefreshRequest::NextFrame);
                            return ReturnData::None;
                        }
                    }
                    Key::Named(NamedKey::ArrowDown) => {
                        if state.selected < state.filtered.len().saturating_sub(1) {
                            state.selected += 1;
                            _ev.request_refresh_all(layershellev::RefreshRequest::NextFrame);
                            return ReturnData::None;
                        }
                    }
                    Key::Character(c) if c == " " => {
                        state.query.push(' ');
                        state.update_filter();
                        _ev.request_refresh_all(layershellev::RefreshRequest::NextFrame);
                        return ReturnData::None;
                    }
                    Key::Character(c) => {
                        if !c.chars().next().map_or(false, |ch| ch.is_control()) {
                            state.query.push_str(c.as_str());
                            state.update_filter();
                            _ev.request_refresh_all(layershellev::RefreshRequest::NextFrame);
                            return ReturnData::None;
                        }
                    }
                    _ => {}
                }
            }
            _ev.request_refresh_all(layershellev::RefreshRequest::NextFrame);
            ReturnData::None
        }
        _ => ReturnData::None,
    })
    .unwrap();
}

fn draw_h_line(pixels: &mut [u32], width: u32, x: u32, y: u32, len: u32, color: u32) {
    for i in x..(x + len).min(width) {
        let idx = (y * width + i) as usize;
        if idx < pixels.len() {
            pixels[idx] = color;
        }
    }
}

fn draw_v_line(pixels: &mut [u32], width: u32, x: u32, y: u32, len: u32, color: u32) {
    for i in y..(y + len) {
        let idx = (i * width + x) as usize;
        if idx < pixels.len() {
            pixels[idx] = color;
        }
    }
}

fn draw_rect(pixels: &mut [u32], width: u32, x: u32, y: u32, w: u32, h: u32, color: u32) {
    draw_h_line(pixels, width, x, y, w, color);
    draw_h_line(pixels, width, x, y + h - 1, w, color);
    draw_v_line(pixels, width, x, y, h, color);
    draw_v_line(pixels, width, x + w - 1, y, h, color);
}

use std::io::Seek;

fn draw(
    tmp: &mut std::fs::File,
    width: u32,
    height: u32,
    state: &PopupState,
    font_system: &mut FontSystem,
    swash_cache: &mut SwashCache,
) {
    let mut pixels = vec![0u32; (width * height) as usize];

    // Background: Catppuccin Macchiato Base (#24273A)
    for p in pixels.iter_mut() {
        *p = 0xFF24273A;
    }

    let metrics = Metrics::new(16.0, 20.0);

    let search_h = 40;
    let list_y = search_h + 10;

    // Draw ratatui-style Search Box
    draw_rect(&mut pixels, width, 0, 0, width, search_h, 0xFF8BD5CA); // Cyan border

    let mut search_title = Buffer::new(font_system, metrics);
    search_title.set_size(font_system, Some(width as f32), Some(20.0));
    search_title.set_text(
        font_system,
        " Search ",
        Attrs::new().color(Color::rgb(0x8B, 0xD5, 0xCA)),
        Shaping::Advanced,
    );
    search_title.shape_until_scroll(font_system, false);

    // Erase top border where title sits
    draw_h_line(&mut pixels, width, 15, 0, 65, 0xFF24273A);
    draw_buffer(
        &mut pixels,
        width,
        height,
        15,
        -10,
        &search_title,
        font_system,
        swash_cache,
    );

    // Draw query
    let mut search_buffer = Buffer::new(font_system, metrics);
    search_buffer.set_size(font_system, Some(width as f32 - 20.0), Some(30.0));
    search_buffer.set_text(
        font_system,
        &state.query,
        Attrs::new().color(Color::rgb(0xCA, 0xD3, 0xF5)),
        Shaping::Advanced,
    );
    search_buffer.shape_until_scroll(font_system, false);
    draw_buffer(
        &mut pixels,
        width,
        height,
        10,
        10,
        &search_buffer,
        font_system,
        swash_cache,
    );

    // Draw ratatui-style Results Box
    let list_h = height - list_y;
    draw_rect(&mut pixels, width, 0, list_y, width, list_h, 0xFF5B6078); // Surface 2 border

    let title_str = format!(
        " Results ({}/{}) ",
        state.filtered.len(),
        state.all_items.len()
    );
    let mut list_title = Buffer::new(font_system, metrics);
    list_title.set_size(font_system, Some(width as f32), Some(20.0));
    list_title.set_text(
        font_system,
        &title_str,
        Attrs::new().color(Color::rgb(0x5B, 0x60, 0x78)),
        Shaping::Advanced,
    );
    list_title.shape_until_scroll(font_system, false);

    // Erase top border for list title
    draw_h_line(&mut pixels, width, 15, list_y, 140, 0xFF24273A);
    draw_buffer(
        &mut pixels,
        width,
        height,
        15,
        list_y as i32 - 10,
        &list_title,
        font_system,
        swash_cache,
    );

    // Draw list
    let mut y = list_y + 10;
    let start_idx = if state.selected >= 10 {
        state.selected - 9
    } else {
        0
    };
    let end_idx = (start_idx + 10).min(state.filtered.len());

    for i in start_idx..end_idx {
        let item = &state.filtered[i];
        let is_selected = i == state.selected;

        if is_selected {
            // Highlight background (#363A4F Surface 0)
            for hy in y..y + 25 {
                if hy >= height - 1 {
                    break;
                }
                draw_h_line(&mut pixels, width, 1, hy, width - 2, 0xFF363A4F);
            }
        }

        let mut item_buffer = Buffer::new(font_system, metrics);
        item_buffer.set_size(font_system, Some(width as f32 - 20.0), Some(25.0));

        let tag_color = match item {
            SuggestionItem::App(_) => Color::rgb(0xA6, 0xDA, 0x95), // Green
            SuggestionItem::Project(_) => Color::rgb(0xC6, 0xA0, 0xF6), // Mauve
        };

        item_buffer.set_rich_text(
            font_system,
            vec![
                (
                    format!("[{}] ", item.tag()).as_str(),
                    Attrs::new()
                        .color(tag_color)
                        .weight(cosmic_text::Weight::BOLD),
                ),
                (
                    item.name(),
                    Attrs::new().color(Color::rgb(0xCA, 0xD3, 0xF5)),
                ),
            ],
            Attrs::new(),
            Shaping::Advanced,
        );
        item_buffer.shape_until_scroll(font_system, false);

        draw_buffer(
            &mut pixels,
            width,
            height,
            10,
            y as i32 + 2,
            &item_buffer,
            font_system,
            swash_cache,
        );
        y += 25;
    }

    let bytes: &[u8] =
        unsafe { std::slice::from_raw_parts(pixels.as_ptr() as *const u8, pixels.len() * 4) };
    tmp.seek(std::io::SeekFrom::Start(0)).unwrap();
    tmp.write_all(bytes).unwrap();
    tmp.flush().unwrap();
}

fn draw_buffer(
    pixels: &mut [u32],
    width: u32,
    height: u32,
    x_offset: i32,
    y_offset: i32,
    buffer: &Buffer,
    font_system: &mut FontSystem,
    swash_cache: &mut SwashCache,
) {
    buffer.draw(
        font_system,
        swash_cache,
        Color::rgb(255, 255, 255),
        |x, y, _w, _h, color| {
            let px = x + x_offset;
            let py = y + y_offset;
            if px >= 0 && px < width as i32 && py >= 0 && py < height as i32 {
                let idx = (py as u32 * width + px as u32) as usize;
                let a = color.a() as f32 / 255.0;
                let bg = pixels[idx];
                let bg_r = ((bg >> 16) & 0xFF) as f32;
                let bg_g = ((bg >> 8) & 0xFF) as f32;
                let bg_b = (bg & 0xFF) as f32;

                let r = (color.r() as f32 * a + bg_r * (1.0 - a)) as u32;
                let g = (color.g() as f32 * a + bg_g * (1.0 - a)) as u32;
                let b = (color.b() as f32 * a + bg_b * (1.0 - a)) as u32;

                pixels[idx] = (0xFF << 24) | (r << 16) | (g << 8) | b;
            }
        },
    );
}
