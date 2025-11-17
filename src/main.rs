use std::{borrow::Cow, io};

use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use ratatui::{
    DefaultTerminal, Frame,
    layout::Rect,
    prelude::*,
    style::Stylize,
    symbols::border,
    text::Line,
    widgets::{Block, Paragraph},
};

#[derive(Debug, Default)]
pub struct App {
    articles: Vec<Article>,
    current_page: usize,
    selected_index: usize, // Index within current page (0-2 for front, 0-3 for regular)
    viewing_article: bool,
    scroll_offset: u16,
    exit: bool,
}

#[derive(Debug, Default)]
pub struct Article {
    title: Cow<'static, str>,
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
        let instructions = if self.viewing_article {
            Line::from(vec![
                " Scroll Down ".into(),
                "<^D>".blue().bold(),
                " Scroll Up ".into(),
                "<^U>".blue().bold(),
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
            let widget = Paragraph::new(vec![
                Line::from(format!("By: {}", article.author)).bold(),
                Line::from(""),
                Line::from(article.main_text.as_ref()),
            ])
            .wrap(ratatui::widgets::Wrap { trim: false })
            .scroll((self.scroll_offset, 0))
            .block(
                Block::bordered()
                    .title(article.title.as_ref())
                    .border_set(border::DOUBLE),
            );
            frame.render_widget(widget, area);
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
                .title(article.title.as_ref())
                .border_set(border::ROUNDED)
                .border_style(Style::default().fg(Color::Blue))
        } else {
            Block::bordered()
                .title(article.title.as_ref())
                .border_set(border::ROUNDED)
        };

        let text = format!("By: {}\n\n{}", article.author, article.main_text);

        Paragraph::new(text)
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
        } else {
            if self.selected_index == 0 {
                self.selected_index = 1;
            } else if self.selected_index == 2 {
                self.selected_index = 3;
            }
        }
    }

    fn move_up(&mut self) {
        if self.current_page == 0 {
            if self.selected_index == 1 || self.selected_index == 2 {
                self.selected_index = 0;
            }
        } else {
            if self.selected_index == 2 {
                self.selected_index = 0;
            } else if self.selected_index == 3 {
                self.selected_index = 1;
            }
        }
    }

    fn move_down(&mut self) {
        if self.current_page == 0 {
            if self.selected_index == 0 {
                self.selected_index = 1;
            }
        } else {
            if self.selected_index == 0 {
                self.selected_index = 2;
            } else if self.selected_index == 1 {
                self.selected_index = 3;
            }
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
    let mut terminal = ratatui::init();
    let mut app = App {
        articles: vec![
            // Front page (3 articles)
            Article {
                title: "Rust 2024 Edition Released".into(),
                author: "Jane Doe".into(),
                main_text: "The Rust team announced the 2024 edition with async improvements, better error messages, and enhanced pattern matching. This release marks a significant milestone in the language's evolution, bringing long-awaited features that developers have been requesting for years. The new async runtime improvements reduce overhead by up to 40%, while the enhanced error messages now provide context-aware suggestions that help developers fix issues faster. Pattern matching has been extended to support more complex scenarios, including nested destructuring and guard clauses that were previously impossible. The edition also introduces new safety guarantees and better tooling integration.\n\nOne of the most anticipated features is the new borrow checker that provides more intelligent analysis of lifetime relationships. This means fewer false positives and more intuitive behavior when working with references. The compiler can now understand more complex borrowing patterns, making it easier to write correct code without fighting the borrow checker.\n\nThe standard library has also received significant updates. New APIs for working with async iterators have been stabilized, making it easier to write streaming applications. The collections module now includes more efficient implementations for common data structures, with some operations seeing up to 30% performance improvements.\n\nError handling has been revolutionized with the new error trait improvements. Stack traces are now more informative, showing the exact path of error propagation through your application. This makes debugging significantly easier, especially in large codebases where errors can originate from deep within the call stack.\n\nThe tooling ecosystem has kept pace with these changes. Cargo now supports workspace inheritance more elegantly, and the new resolver algorithm handles dependency conflicts more gracefully. The documentation generator has been enhanced to produce more readable and navigable documentation.\n\nCommunity response has been overwhelmingly positive, with many projects already planning their migration to the new edition. The backwards compatibility story remains strong, ensuring that existing code continues to work while new features become available.".into(),
            },
            Article {
                title: "TUI Apps Are Back".into(),
                author: "John Smith".into(),
                main_text: "Developers are rediscovering the power of terminal applications. With modern libraries like Ratatui, creating beautiful and functional TUIs has never been easier. The resurgence is driven by remote work trends and the need for lightweight, fast applications that work over SSH. Many companies are now building internal tools as TUIs, finding them more efficient than web interfaces for certain workflows. The ecosystem has matured significantly, with comprehensive widget libraries and cross-platform support.".into(),
            },
            Article {
                title: "Ratatui 1.0 Coming Soon".into(),
                author: "Alice Chen".into(),
                main_text: "After years of development, Ratatui approaches stable release. The library has become the de facto standard for building terminal user interfaces in Rust. Version 1.0 will bring API stability guarantees, improved performance, and a wealth of new widgets. The maintainers have worked tirelessly to ensure backwards compatibility while adding powerful new features. Documentation has been completely rewritten with extensive examples and tutorials.".into(),
            },
            // Page 1 (4 articles)
            Article {
                title: "Cross-Platform CLI Tools".into(),
                author: "Bob Wilson".into(),
                main_text: "Modern CLI tools leverage Rust's cross-compilation capabilities. Developers can now write tools once and compile them for Windows, macOS, and Linux without modification. This has led to an explosion of high-quality command-line utilities that work consistently across all platforms. The tooling has improved dramatically, with cargo-cross making it trivial to target different architectures. Binary sizes have also decreased thanks to better optimization passes.".into(),
            },
            Article {
                title: "WebAssembly in 2024".into(),
                author: "Sarah Lee".into(),
                main_text: "WebAssembly adoption accelerates across industries. What started as a browser technology has expanded to server-side computing, edge functions, and plugin systems. Major cloud providers now offer WASM runtimes, and the component model promises true language interoperability. Performance continues to improve, with some benchmarks showing near-native speeds. The security model makes it ideal for sandboxed execution environments.".into(),
            },
            Article {
                title: "Git 3.0 Preview".into(),
                author: "Mike Brown".into(),
                main_text: "Git maintainers reveal plans for version 3.0 features. The upcoming release focuses on performance improvements for large repositories and better handling of binary files. New features include native support for partial clones, improved merge algorithms, and a redesigned staging area. The command-line interface will remain backwards compatible, but internal data structures are being optimized for modern storage systems.".into(),
            },
            Article {
                title: "AI Code Assistants".into(),
                author: "Lisa Wang".into(),
                main_text: "AI tools are reshaping how developers write code. From autocomplete to full function generation, these assistants are becoming indispensable. Studies show productivity gains of 30-50% for certain tasks, though concerns about code quality and security remain. The technology continues to evolve rapidly, with newer models understanding context better and generating more idiomatic code. Integration with IDEs has become seamless.".into(),
            },
            // Page 2 (4 articles)
            Article {
                title: "Linux Kernel 7.0".into(),
                author: "Tom Davis".into(),
                main_text: "The latest kernel brings significant performance gains. Memory management has been overhauled, reducing allocation overhead by 25%. The scheduler now better handles heterogeneous CPU architectures common in modern processors. Filesystem performance has improved across the board, with ext4 and btrfs seeing major optimizations. Network stack changes reduce latency for high-frequency trading and gaming workloads.".into(),
            },
            Article {
                title: "Docker Alternatives".into(),
                author: "Emma Scott".into(),
                main_text: "Podman and other tools challenge Docker's dominance. The container ecosystem has matured beyond a single vendor, with alternatives offering rootless containers, better security, and Kubernetes compatibility. Organizations are evaluating their options based on specific use cases. Some prefer the daemonless architecture of Podman, while others value the ecosystem around Docker. The OCI standard ensures compatibility.".into(),
            },
            Article {
                title: "Terminal Emulators".into(),
                author: "Chris Martin".into(),
                main_text: "Modern terminals leverage GPU for better performance. Applications like Alacritty and Kitty have popularized GPU rendering, making scrolling smooth and responsive even with thousands of lines. Font rendering has improved dramatically, with proper ligature support and emoji rendering. Customization options have expanded, allowing developers to create personalized environments. Performance benchmarks show 10x improvements over traditional terminals.".into(),
            },
            Article {
                title: "Neovim 1.0".into(),
                author: "Anna Kim".into(),
                main_text: "Neovim celebrates its first stable major release. The editor has successfully modernized Vim while maintaining compatibility. Lua scripting has enabled a rich plugin ecosystem, and the built-in LSP client provides IDE-like features. Tree-sitter integration offers fast and accurate syntax highlighting. The community has grown exponentially, with thousands of plugins available. Performance remains excellent even with heavy customization.".into(),
            },
        ],
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
