use std::{fs::{self, read_dir}, io::ErrorKind, path::{Path, PathBuf}, usize::MAX};

use chrono::{DateTime, Utc};
use ratatui::{DefaultTerminal, Frame, buffer::Buffer, crossterm::{event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyEventState, KeyModifiers}, terminal::disable_raw_mode}, layout::{Constraint, Rect, Rows}, style::{Color, Modifier, Style}, text::{self, Line, Text}, widgets::{self, Borders, List, ScrollbarOrientation, ScrollbarState, StatefulWidget, Table, TableState, Widget}};

use crate::{scanner::Scanner, snapshots::{Snapshot, Snapshots, bytes_to_units}};

const CURRENT_DEPTH: usize = 1;
const MAX_DEPTH: usize = 100000;

enum CurrentScreen {
    FileExplorer,
    Comparison,
}

pub struct App {
    screen: CurrentScreen,

    root_path: PathBuf,

    scanner: Scanner,
    snapshots: Snapshots,   
    pub ss_state: TableState,
    pub ss_scroll_state: ScrollbarState,
    pub cmp_state: TableState,
    pub cmp_scroll_state: ScrollbarState,
    pub cmp_len: usize,
    exit: bool,
    
    show_save_popup: bool,
}

impl App {
    pub fn new(args: &crate::Args) -> Self {
        let path = args.path.clone();
        let scanner = Scanner::new();
        let snapshots = scanner.scan(&path, CURRENT_DEPTH, MAX_DEPTH);

        match snapshots {
            Ok(snapshots) => {
                let ss_state = TableState::default().with_selected(0);
                let ss_scroll_state = ScrollbarState::new(snapshots.snapshots.len());
                let cmp_state = TableState::default().with_selected(0);
                // length will be modified later in the code
                let cmp_scroll_state = ScrollbarState::new(0);
                Self {
                    screen: CurrentScreen::FileExplorer,
                    root_path: path,

                    scanner,
                    snapshots,
                    ss_state,
                    ss_scroll_state,
                    cmp_state,
                    cmp_scroll_state,
                    cmp_len: 0, // will be modified later on
                    exit: false,

                    show_save_popup: false,
                }
            }
            Err(e) => panic!("Error: {:?}", e.raw_os_error()),
        }
    }

    pub fn ss_next(&mut self) {
        let i = match self.ss_state.selected() {
            Some(i) => {
                if i >= self.snapshots.snapshots.len() - 1 {
                    0
                } else {
                    i + 1
                }
            }
            None => 0
        };
        self.ss_state.select(Some(i));
        self.ss_scroll_state = self.ss_scroll_state.position(i);
    }

    pub fn ss_previous(&mut self) {
        let i = match self.ss_state.selected() {
            Some(i) => {
                if i <= 0 {
                    self.snapshots.snapshots.len() - 1
                } else {
                    i - 1
                }
            }
            None => 0
        };
        self.ss_state.select(Some(i));
        self.ss_scroll_state = self.ss_scroll_state.position(i);
    }

    pub fn cmp_next(&mut self) {
        let i = match self.cmp_state.selected() {
            Some(i) => {
                if i >= self.snapshots.snapshots.len() - 1 {
                    0
                } else {
                    i + 1
                }
            }
            None => 0
        };
        self.cmp_state.select(Some(i));
        self.cmp_scroll_state = self.cmp_scroll_state.position(i);
    }

    pub fn cmp_previous(&mut self) {
        let i = match self.cmp_state.selected() {
            Some(i) => {
                if i <= 0 {
                    self.cmp_len - 1
                } else {
                    i - 1
                }
            }
            None => 0
        };
        self.cmp_state.select(Some(i));
        self.cmp_scroll_state = self.cmp_scroll_state.position(i);
    }

    fn handle_save_popup(&self, area: Rect, buf: &mut Buffer) {
        if self.show_save_popup {
            let centered_area = area.centered(Constraint::Percentage(40), Constraint::Percentage(20));
            let popup_area = Rect::new(centered_area.x, centered_area.y, centered_area.width, centered_area.height);
            let popup_block = widgets::Block::new()
                .title_top("Are you sure you want to save?")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Black))
                .style(Style::default().bg(Color::White).fg(Color::Black));

            let input = widgets::Paragraph::new(vec![
                Line::from("[Y]es | [N]o")
            ])
                .block(popup_block);

            Widget::render(input, popup_area, buf);
        }
    }

    pub fn save(&mut self) -> Result<(), std::io::Error> {
        let home = home::home_dir().unwrap();
        let dir_path = home.join(".ddisk");
        self.snapshots.save(dir_path)?;
        Ok(())
    }

    fn go_to_parent(&mut self) {
        // in theory, every snapshot in one frame should have the same parent
let     parent_path = &self.snapshots.snapshots[0].parent_path.clone();

        match parent_path {
            Some(root) => {
                let snapshots = self.scanner.scan(root, CURRENT_DEPTH, MAX_DEPTH);
                match snapshots {
                    Ok(snapshots) => {
                        self.snapshots = snapshots;
                        self.root_path = root.to_path_buf();
                    }
                    Err(e) => {
                        if e.kind() == ErrorKind::NotFound {
                            panic!("Parent dir not found. {:?}", parent_path);
                        }
                    }
                }
            }
            None => {
                let ss = &self.snapshots.snapshots[0];
                if ss.path == PathBuf::from("/") {
                    eprintln!("Couldn't get parent since this is the `/` directory");
                    panic!()
                }
            }
        }
    }

    fn go_to_child(&mut self) {
        let ss_idx = self.ss_state.selected().unwrap();
        let ss = &self.snapshots.snapshots[ss_idx].clone();
        
        if ss.is_dir {
            let path = &ss.path;
            let snapshots = self.scanner.scan(path, CURRENT_DEPTH, MAX_DEPTH).unwrap();
            self.snapshots = snapshots;
            self.root_path = path.to_path_buf();
        }
    }

    fn toggle_screen(&mut self) {
        let screen = match self.screen {
            CurrentScreen::FileExplorer => CurrentScreen::Comparison,
            CurrentScreen::Comparison => CurrentScreen::FileExplorer,
        };
        self.screen = screen;
    }

    /// runs the application's main loop until the user quits
    pub fn run(&mut self, terminal: &mut DefaultTerminal) -> std::io::Result<()> {
        while !self.exit {
            terminal.draw(|frame| self.draw(frame))?;
            self.handle_events()?;
        }
        terminal.clear()?;
        disable_raw_mode()?;
        Ok(())
    }
    
    fn draw(&mut self, frame: &mut Frame) {
        frame.render_widget(self, frame.area());
    }

    fn handle_events(&mut self) -> std::io::Result<()> {
        match event::read()? {
            Event::Key(key_event) if key_event.kind == KeyEventKind::Press => {
                self.handle_key_event_press(key_event)?;
            }
            _ => ()
        }

        Ok(())
    }

    fn handle_key_event_press(&mut self, key_event: KeyEvent) -> std::io::Result<()> {
        if self.show_save_popup {
            match key_event.code {
                KeyCode::Enter | KeyCode::Char('y') => {
                    self.show_save_popup = false;
                    self.save()?;
                }
                KeyCode::Esc | KeyCode::Char('n') => self.show_save_popup = false,
                _ => ()
            }
        } else {
            match key_event.code {
                KeyCode::Char('q') => self.exit = true,
                KeyCode::Char('s') => self.show_save_popup = true,
                KeyCode::Char('c') => self.toggle_screen(),
                KeyCode::Char('j') | KeyCode::Down => self.ss_next(),
                KeyCode::Char('k') | KeyCode::Up => self.ss_previous(),
                KeyCode::Backspace | KeyCode::Delete => self.go_to_parent(),
                KeyCode::Enter => self.go_to_child(),
                _ => ()
            }
        }

        Ok(())
    }

    fn render_main(&self, area: ratatui::prelude::Rect, buf: &mut ratatui::prelude::Buffer) {
        let title = Line::from(self.root_path.to_str().unwrap());
        // let total_size: u64 = self.snapshots.clone().into_iter().map(|ss| {
        //     ss.size
        // })
        // .sum();
        let total_size = self.snapshots.size;
    
        let extra_info_line = vec![
            text::Span::from(" Total disk usage "),
            text::Span::from(bytes_to_units(total_size)),
            text::Span::from(" "),
        ];
        let extra_info = text::Line::from(extra_info_line)
            .style(
                Style::default()
                    .bg(Color::White)
                    .fg(Color::Black)
            );
        let block = widgets::Block::bordered()
            .title(title.centered())
            .title_bottom(extra_info);
        let rows = self.snapshots.to_rows_unit();

        let table = widgets::Table::new(rows, [
            Constraint::Percentage(30),
            Constraint::Percentage(70),
        ])
            .block(block)
            .header(widgets::Row::new(vec![
                widgets::Cell::new("File Name"),
                widgets::Cell::new("Size"),
            ]))
            .row_highlight_style(Style::default().add_modifier(Modifier::REVERSED));

        let scrollbar = widgets::Scrollbar::new(ScrollbarOrientation::VerticalRight)
            .begin_symbol(Some("▲"))
            .end_symbol(Some("▼"));

        let mut table_state = self.ss_state.clone();
        let mut scroll_state = self.ss_scroll_state.clone();

        StatefulWidget::render(table, area, buf, &mut table_state);
        StatefulWidget::render(scrollbar, area, buf, &mut scroll_state);
            
        self.handle_save_popup(area, buf);
    }

    fn render_comparison(&mut self, area: ratatui::prelude::Rect, buf: &mut ratatui::prelude::Buffer) {
        let home = home::home_dir().unwrap();
        let dir = home.join(".ddisk");

        let mut old_snapshotss = Vec::new();

        // collecting json to read
        for file in read_dir(dir).unwrap() {
            let file = file.unwrap();
            if file.path().extension().unwrap() != "json" {
                continue
            }
            let snapshots = self.snapshots.load(file.path()).unwrap();
            if let Some(path) = &snapshots.path {
                // snapshots belongs to this specific path
                if *path == self.root_path {
                    old_snapshotss.push(snapshots);
                }
            }
        }

        let cmp_len = old_snapshotss.len();
        self.cmp_len = cmp_len;
        self.cmp_scroll_state = self.cmp_scroll_state.content_length(cmp_len);

        let mut comparisons = Vec::new();

        for old_snapshots in &old_snapshotss {
            match self.snapshots.compare_growth_to_rows_unit(&old_snapshots) {
                Ok(rows) => comparisons.push(rows),
                Err(e) => panic!("{e}"), 
            }
        }

        let title = Line::from(format!("Comparison of {:?}", self.root_path));

        let block = widgets::Block::bordered()
            .title(title.centered());

        let comparison = comparisons.get(self.cmp_state.selected().unwrap());
        if let Some(comparison) = comparison {
            let table = widgets::Table::new(comparison.iter().cloned(), [
                Constraint::Percentage(30),
                Constraint::Percentage(70),
            ])
                .block(block)
                .header(widgets::Row::new(vec![
                    widgets::Cell::new("File Path"),
                    widgets::Cell::new("Size Difference"),
                ]))
                .row_highlight_style(Style::default().add_modifier(Modifier::REVERSED));

            let scrollbar = widgets::Scrollbar::new(ScrollbarOrientation::VerticalRight)
                .begin_symbol(Some("▲"))
                .end_symbol(Some("▼"));

            let mut table_state = self.cmp_state.clone();
            let mut scroll_state = self.cmp_scroll_state.clone();

            StatefulWidget::render(table, area, buf, &mut table_state);
            StatefulWidget::render(scrollbar, area, buf, &mut scroll_state);
        }
    }
}

impl Widget for &mut App {
    fn render(self, area: ratatui::prelude::Rect, buf: &mut ratatui::prelude::Buffer)
    where
        Self: Sized {
        match self.screen {
            CurrentScreen::FileExplorer => self.render_main(area, buf),
            CurrentScreen::Comparison => self.render_comparison(area, buf)
        }
    }
}