use std::io;

use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use ratatui::{
    DefaultTerminal, Frame,
    layout::Rect,
    prelude::*,
    style::Stylize,
    symbols::border,
    text::{Line, Text},
    widgets::{Block, Paragraph},
};
use serde::Deserialize;

#[derive(Debug, Default)]
pub struct App {
    articles: Vec<Article>,
    current_page: usize,
    selected_index: usize, // Index within current page (0-2 for front, 0-3 for regular)
    viewing_article: bool,
    scroll_offset: u16,
    exit: bool,
}

#[derive(Debug, Default, Deserialize)]
pub struct Article {
    title: String,
    author: String,
    content: String,
}

fn fetch_articles() -> Result<Vec<Article>, Box<dyn std::error::Error>> {
    dotenvy::dotenv().ok();
    let backend_url = std::env::var("BACKEND_URL")?;
    let url = format!("{}/articles", backend_url);
    Ok(reqwest::blocking::get(&url)?.json()?)
}

impl App {
    pub fn run(&mut self, terminal: &mut DefaultTerminal) -> io::Result<()> {
        while !self.exit {
            terminal.draw(|frame| self.draw(frame))?;
            self.handle_events()?;
        }
        Ok(())
    }

    fn draw(&self, frame: &mut Frame) {
        // Outer block with title and instructions
        let title = Line::from(vec![
            " ".into(),
            "De".blue().bold(),
            "veloper ".bold(),
            "Ne".blue().bold(),
            "ws ".bold(),
            "TUI".blue().bold(),
            " ".into(),
        ]);
        let instructions = if self.viewing_article {
            Line::from(vec![
                " Scroll Down ".into(),
                "<^d>".blue().bold(),
                " Scroll Up ".into(),
                "<^u>".blue().bold(),
                " Back ".into(),
                "<Esc> ".blue().bold(),
            ])
        } else {
            Line::from(vec![
                " Prev Page ".into(),
                "<H>".blue().bold(),
                " Next Page ".into(),
                "<L>".blue().bold(),
                " Quit ".into(),
                "<q> ".blue().bold(),
                " Move ←,↓,↑,→ ".into(),
                "<h,j,k,l>".blue().bold(),
                " Open ".into(),
                "<Enter>".blue().bold(),
            ])
        };
        let outer_block = Block::bordered()
            .title(title.centered())
            .title_bottom(instructions.centered())
            .border_set(border::THICK);

        // Inner area
        let inner_area = outer_block.inner(frame.area());

        // Render outer block
        frame.render_widget(outer_block, frame.area());

        if self.viewing_article {
            self.draw_single_article(frame, inner_area);
        } else if self.current_page == 0 {
            self.draw_front_page(frame, inner_area);
        } else {
            self.draw_regular_page(frame, inner_area);
        }
    }

    fn draw_front_page(&self, frame: &mut Frame, area: Rect) {
        // Top: big main article, Bottom: two side-by-side
        let vertical = Layout::default()
            .direction(Direction::Vertical)
            .constraints(vec![Constraint::Percentage(60), Constraint::Percentage(40)])
            .split(area);

        // Top article (main news)
        if let Some(article) = self.articles.first() {
            frame.render_widget(
                self.article_widget(article, self.selected_index == 0),
                vertical[0],
            );
        }

        // Bottom two side-by-side
        let horizontal = Layout::default()
            .direction(Direction::Horizontal)
            .constraints(vec![Constraint::Percentage(50), Constraint::Percentage(50)])
            .split(vertical[1]);

        if let Some(article) = self.articles.get(1) {
            frame.render_widget(
                self.article_widget(article, self.selected_index == 1),
                horizontal[0],
            );
        }
        if let Some(article) = self.articles.get(2) {
            frame.render_widget(
                self.article_widget(article, self.selected_index == 2),
                horizontal[1],
            );
        }
    }

    fn draw_regular_page(&self, frame: &mut Frame, area: Rect) {
        // 2x2 grid
        let vertical = Layout::default()
            .direction(Direction::Vertical)
            .constraints(vec![Constraint::Percentage(50), Constraint::Percentage(50)])
            .split(area);

        let top_row = Layout::default()
            .direction(Direction::Horizontal)
            .constraints(vec![Constraint::Percentage(50), Constraint::Percentage(50)])
            .split(vertical[0]);

        let bottom_row = Layout::default()
            .direction(Direction::Horizontal)
            .constraints(vec![Constraint::Percentage(50), Constraint::Percentage(50)])
            .split(vertical[1]);

        // Page 1 starts at article index 3, page 2 at index 7, etc.
        let start_idx = 3 + (self.current_page - 1) * 4;

        let positions = [top_row[0], top_row[1], bottom_row[0], bottom_row[1]];
        for (i, &pos) in positions.iter().enumerate() {
            if let Some(article) = self.articles.get(start_idx + i) {
                frame.render_widget(self.article_widget(article, self.selected_index == i), pos);
            }
        }
    }

    fn draw_single_article(&self, frame: &mut Frame, area: Rect) {
        let article_idx = self.get_selected_article_index();
        if let Some(article) = self.articles.get(article_idx) {
            let block = Block::bordered()
                .border_set(border::ROUNDED)
                .border_style(Style::default().fg(Color::Blue));
            let inner = block.inner(area);
            frame.render_widget(block, area);

            // Build full content with title, author, and markdown
            let mut lines = vec![
                Line::from(article.title.as_str()).style(Style::default().fg(Color::Cyan).bold()),
                Line::from(""),
                Line::from(format!("By: {}", article.author))
                    .style(Style::default().fg(Color::Yellow)),
                Line::from(""),
            ];

            // Add markdown-rendered content
            let markdown_text = tui_markdown::from_str(&article.content);
            lines.extend(markdown_text.lines);

            let widget = Paragraph::new(Text::from(lines))
                .wrap(ratatui::widgets::Wrap { trim: false })
                .scroll((self.scroll_offset, 0));
            frame.render_widget(widget, inner);
        }
    }

    fn get_selected_article_index(&self) -> usize {
        if self.current_page == 0 {
            self.selected_index
        } else {
            3 + (self.current_page - 1) * 4 + self.selected_index
        }
    }

    fn article_widget<'a>(&self, article: &'a Article, selected: bool) -> Paragraph<'a> {
        let block = if selected {
            Block::bordered()
                .border_set(border::ROUNDED)
                .border_style(Style::default().fg(Color::Blue))
        } else {
            Block::bordered().border_set(border::ROUNDED)
        };

        // Build text with styled title and author, then markdown content
        let mut lines = vec![
            Line::from(article.title.as_str()).style(Style::default().fg(Color::Cyan).bold()),
            Line::from(""),
            Line::from(format!("By: {}", article.author)).style(Style::default().fg(Color::Yellow)),
            Line::from(""),
        ];

        // Add markdown-rendered content
        let markdown_text = tui_markdown::from_str(&article.content);
        lines.extend(markdown_text.lines);

        Paragraph::new(Text::from(lines))
            .wrap(ratatui::widgets::Wrap { trim: false })
            .block(block)
    }

    fn handle_events(&mut self) -> io::Result<()> {
        match event::read()? {
            // it's important to check that the event is a key press event as
            // crossterm also emits key release and repeat events on Windows.
            Event::Key(key_event) if key_event.kind == KeyEventKind::Press => {
                self.handle_key_event(key_event)
            }
            _ => {}
        };
        Ok(())
    }

    fn handle_key_event(&mut self, key_event: KeyEvent) {
        if self.viewing_article {
            match key_event.code {
                KeyCode::Esc | KeyCode::Char('q') => {
                    self.viewing_article = false;
                    self.scroll_offset = 0;
                }
                KeyCode::Char('d') if key_event.modifiers.contains(KeyModifiers::CONTROL) => {
                    self.scroll_down();
                }
                KeyCode::Char('u') if key_event.modifiers.contains(KeyModifiers::CONTROL) => {
                    self.scroll_up();
                }
                _ => {}
            }
        } else {
            match key_event.code {
                KeyCode::Char('q') => self.exit(),
                KeyCode::Char('L') => self.next_page(),
                KeyCode::Char('H') => self.prev_page(),
                KeyCode::Char('h') => self.move_left(),
                KeyCode::Char('j') => self.move_down(),
                KeyCode::Char('k') => self.move_up(),
                KeyCode::Char('l') => self.move_right(),
                KeyCode::Enter => self.open_article(),
                _ => {}
            }
        }
    }

    fn exit(&mut self) {
        self.exit = true;
    }

    fn open_article(&mut self) {
        self.viewing_article = true;
        self.scroll_offset = 0;
    }

    fn scroll_down(&mut self) {
        self.scroll_offset = self.scroll_offset.saturating_add(5);
    }

    fn scroll_up(&mut self) {
        self.scroll_offset = self.scroll_offset.saturating_sub(5);
    }

    fn move_left(&mut self) {
        if self.current_page == 0 {
            // Front page: 0 (top), 1 (bottom-left), 2 (bottom-right)
            if self.selected_index == 2 {
                self.selected_index = 1;
            }
        } else {
            // Regular page: 0 (top-left), 1 (top-right), 2 (bottom-left), 3 (bottom-right)
            if self.selected_index == 1 {
                self.selected_index = 0;
            } else if self.selected_index == 3 {
                self.selected_index = 2;
            }
        }
    }

    fn move_right(&mut self) {
        if self.current_page == 0 {
            if self.selected_index == 1 {
                self.selected_index = 2;
            }
        } else if self.selected_index == 0 {
            self.selected_index = 1;
        } else if self.selected_index == 2 {
            self.selected_index = 3;
        }
    }

    fn move_up(&mut self) {
        if self.current_page == 0 {
            if self.selected_index == 1 || self.selected_index == 2 {
                self.selected_index = 0;
            }
        } else if self.selected_index == 2 {
            self.selected_index = 0;
        } else if self.selected_index == 3 {
            self.selected_index = 1;
        }
    }

    fn move_down(&mut self) {
        if self.current_page == 0 {
            if self.selected_index == 0 {
                self.selected_index = 1;
            }
        } else if self.selected_index == 0 {
            self.selected_index = 2;
        } else if self.selected_index == 1 {
            self.selected_index = 3;
        }
    }

    fn next_page(&mut self) {
        let max_page = self.max_page();
        if self.current_page < max_page {
            self.current_page += 1;
            self.selected_index = 0; // Reset selection on page change
        }
    }

    fn prev_page(&mut self) {
        if self.current_page > 0 {
            self.current_page -= 1;
            self.selected_index = 0; // Reset selection on page change
        }
    }

    fn max_page(&self) -> usize {
        if self.articles.len() <= 3 {
            0
        } else {
            1 + (self.articles.len() - 3).saturating_sub(1) / 4
        }
    }
}

fn main() -> io::Result<()> {
    let articles = match fetch_articles() {
        Ok(articles) => articles,
        Err(e) => {
            eprintln!("Failed to fetch articles: {}", e);
            return Ok(());
        }
    };

    if articles.is_empty() {
        eprintln!("No articles available");
        return Ok(());
    }

    let mut terminal = ratatui::init();
    let mut app = App {
        articles,
        current_page: 0,
        selected_index: 0,
        viewing_article: false,
        scroll_offset: 0,
        exit: false,
    };
    let app_result = app.run(&mut terminal);
    ratatui::restore();
    app_result
}
