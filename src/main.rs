use std::{borrow::Cow, io};

use crossterm::{
    event::{self, Event, KeyCode, KeyEvent, KeyEventKind},
    terminal,
};
use ratatui::{
    DefaultTerminal, Frame,
    buffer::Buffer,
    layout::Rect,
    prelude::*,
    style::Stylize,
    symbols::border,
    text::{Line, Text},
    widgets::{Block, Paragraph, Widget},
};

#[derive(Debug, Default)]
pub struct App {
    articles: Vec<Article>,
    exit: bool,
}

#[derive(Debug, Default)]
pub struct Article {
    title: Cow<'static, str>,
    description: Cow<'static, str>,
    author: Cow<'static, str>,
    main_text: Cow<'static, str>,
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
        let title = Line::from(" Developer News ".bold());
        let instructions = Line::from(vec![" Quit ".into(), "<Q> ".blue().bold()]);
        let outer_block = Block::bordered()
            .title(title.centered())
            .title_bottom(instructions.centered())
            .border_set(border::THICK);

        // Inner area
        let inner_area = outer_block.inner(frame.area());

        // Render outer block
        frame.render_widget(outer_block, frame.area());

        // Create constraints for each article (equal height)
        let constraints: Vec<Constraint> = self
            .articles
            .iter()
            .map(|_| Constraint::Ratio(1, self.articles.len() as u32))
            .collect();

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints(constraints)
            .split(inner_area);

        // Render each article
        for (i, article) in self.articles.iter().enumerate() {
            let article_widget = Paragraph::new(vec![
                Line::from(article.description.as_ref()).italic(),
                Line::from(format!("By: {}", article.author)),
                Line::from(""),
                Line::from(article.main_text.as_ref()),
            ])
            .block(
                Block::bordered()
                    .title(article.title.as_ref())
                    .border_set(border::ROUNDED),
            );

            frame.render_widget(article_widget, chunks[i]);
        }
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
        match key_event.code {
            KeyCode::Char('q') => self.exit(),
            _ => {}
        }
    }

    fn exit(&mut self) {
        self.exit = true;
    }
}

fn main() -> io::Result<()> {
    let mut terminal = ratatui::init();
    let mut app = App {
        articles: vec![
            Article {
                title: "Rust 2024 Edition Released".into(),
                description: "New features and improvements".into(),
                author: "Jane Doe".into(),
                main_text: "The Rust team announced...".into(),
            },
            Article {
                title: "TUI Apps Are Back".into(),
                description: "Terminal UIs gaining popularity".into(),
                author: "John Smith".into(),
                main_text: "Developers are rediscovering...".into(),
            },
            Article {
                title: "Ratatui 1.0 Coming Soon".into(),
                description: "Major milestone for the library".into(),
                author: "Alice Chen".into(),
                main_text: "After years of development...".into(),
            },
            Article {
                title: "Cross-Platform CLI Tools".into(),
                description: "Building once, running everywhere".into(),
                author: "Bob Wilson".into(),
                main_text: "Modern CLI tools are...".into(),
            },
        ],
        exit: false,
    };
    let app_result = app.run(&mut terminal);
    ratatui::restore();
    app_result
}
