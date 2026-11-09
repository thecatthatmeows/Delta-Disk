use std::{io::ErrorKind, path::PathBuf, usize::MAX};

use ratatui::{DefaultTerminal, Frame, buffer::Buffer, crossterm::{event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyEventState, KeyModifiers}, terminal::disable_raw_mode}, layout::{Constraint, Rect}, style::{Color, Modifier, Style}, text::{self, Line}, widgets::{self, Borders, ScrollbarOrientation, ScrollbarState, StatefulWidget, TableState, Widget}};
use tui_input::Input;

use crate::{scanner::Scanner, snapshots::{Snapshots, bytes_to_units}};

const CURRENT_DEPTH: usize = 1;
const MAX_DEPTH: usize = 100000;

pub struct App {
    root_path: PathBuf,

    scanner: Scanner,
    snapshots: Snapshots,   
    pub state: TableState,
    pub scroll_state: ScrollbarState,
    exit: bool,
    
    input: Input,
    show_save_popup: bool,
    should_save: bool,
}

impl App {
    pub fn new(args: &crate::Args) -> Self {
        let path = args.path.clone();
        let scanner = Scanner::new();
        let snapshots = scanner.scan(&path, CURRENT_DEPTH, MAX_DEPTH);

        match snapshots {
            Ok(snapshots) => {
                let state = TableState::default().with_selected(0);
                let scroll_state = ScrollbarState::new(snapshots.snapshots.len());
                Self {
                    root_path: path,

                    scanner,
                    snapshots,
                    state,
                    scroll_state,
                    exit: false,

                    input: Input::default(),
                    show_save_popup: false,
                    should_save: false,
                }
            }
            Err(e) => panic!("Error: {:?}", e.raw_os_error()),
        }
    }

    pub fn next(&mut self) {
        let i = match self.state.selected() {
            Some(i) => {
                if i >= self.snapshots.snapshots.len() - 1 {
                    0
                } else {
                    i + 1
                }
            }
            None => 0
        };
        self.state.select(Some(i));
        self.scroll_state = self.scroll_state.position(i);
    }

    pub fn previous(&mut self) {
        let i = match self.state.selected() {
            Some(i) => {
                if i <= 0 {
                    self.snapshots.snapshots.len() - 1
                } else {
                    i - 1
                }
            }
            None => 0
        };
        self.state.select(Some(i));
        self.scroll_state = self.scroll_state.position(i);
    }

    fn handle_save_popup(&self, area: Rect, buf: &mut Buffer) {
        if self.show_save_popup {
            let centered_area = area.centered(Constraint::Percentage(40), Constraint::Percentage(20));
            let popup_area = Rect::new(centered_area.x, centered_area.y, centered_area.width, centered_area.height);
            let popup_block = widgets::Block::new()
                .title("Are you sure you want to save?")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Black))
                .style(Style::default().bg(Color::White).fg(Color::Black));

            let input = widgets::Paragraph::new(self.input.value())
                .block(popup_block);

            Widget::render(input, popup_area, buf);
        }
    }

    fn go_to_parent(&mut self) {
        // in theory, every snapshot in one frame should have the same parent
        let ss = &self.snapshots.snapshots[0];
        let parent_path = &ss.parent_path;

        match parent_path {
            Some(root) => {
                let snapshots = self.scanner.scan(root, CURRENT_DEPTH, MAX_DEPTH);
                match snapshots {
                    Ok(snapshots) => self.snapshots = snapshots,
                    Err(e) => {
                        if e.kind() == ErrorKind::NotFound {
                            panic!("Parent dir not found. {:?}", parent_path);
                        }
                    }
                }
            }
            None => {
                if ss.path == PathBuf::from("/") {
                    eprintln!("Couldn't get parent since this is the `/` directory");
                    panic!()
                }
            }
        }
    }

    fn go_to_child(&mut self) {
        let ss_idx = self.state.selected().unwrap();
        let ss = &self.snapshots.snapshots[ss_idx];
        
        if ss.is_dir {
            let path = &ss.path;
            let snapshots = self.scanner.scan(path, CURRENT_DEPTH, MAX_DEPTH).unwrap();
            self.snapshots = snapshots;
        }
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
    
    fn draw(&self, frame: &mut Frame) {
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
                KeyCode::Enter => {
                    self.should_save = true;
                    self.show_save_popup = false;
                }
                KeyCode::Esc => self.show_save_popup = false,
                _ => ()
            }
        } else {
            match key_event.code {
                KeyCode::Char('q') => self.exit = true,
                KeyCode::Char('s') => self.show_save_popup = true,
                KeyCode::Char('j') | KeyCode::Down => self.next(),
                KeyCode::Char('k') | KeyCode::Up => self.previous(),
                KeyCode::Backspace | KeyCode::Delete => self.go_to_parent(),
                KeyCode::Enter => self.go_to_child(),
                _ => ()
            }
        }

        Ok(())
    }
}

impl Widget for &App {
    fn render(self, area: ratatui::prelude::Rect, buf: &mut ratatui::prelude::Buffer)
    where
        Self: Sized {
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
            Constraint::Percentage(20),
            Constraint::Percentage(80),
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

        let mut table_state = self.state.clone();
        let mut scroll_state = self.scroll_state.clone();

        StatefulWidget::render(table, area, buf, &mut table_state);
        StatefulWidget::render(scrollbar, area, buf, &mut scroll_state);
            
        self.handle_save_popup(area, buf);
    }
}