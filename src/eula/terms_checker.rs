use crate::util::file_util::{load_file, save_file};
use crossterm::event::{self, Event, KeyCode};
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, Paragraph},
};
use std::time::Duration;
use std::time::{SystemTime, UNIX_EPOCH};

pub struct ConsentManager;
impl ConsentManager {
    pub fn check() -> (bool, bool) {
        let file = load_file("", "agreements");
        let existing = ConsentUiState::from_str(&file).sanitize();

        let final_state = if existing.eula {
            existing
        } else {
            let choice = run_consent_ui();
            let state = match choice {
                UserChoice::Deny => ConsentUiState::denied(),
                UserChoice::AcceptEULA => ConsentUiState {
                    eula: true,
                    tos: false,
                    pp: false,
                    focus: Focus::Cancel,
                },
                UserChoice::AcceptAll => ConsentUiState {
                    eula: true,
                    tos: true,
                    pp: true,
                    focus: Focus::Cancel,
                },
            };
            let state = state.sanitize();
            save_file("", "agreements", &state.to_string());
            state
        };

        (final_state.eula, final_state.pp && final_state.tos)
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum UserChoice {
    Deny,
    AcceptEULA,
    AcceptAll,
}

#[derive(Debug, Clone, Copy)]
struct ConsentUiState {
    eula: bool,
    tos: bool,
    pp: bool,
    focus: Focus,
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum Focus {
    Eula,
    Tos,
    Pp,
    Cancel,
    Continue,
    ContinueAll,
}

impl ConsentUiState {
    fn denied() -> Self {
        Self {
            eula: false,
            pp: false,
            tos: false,
            focus: Focus::Cancel,
        }
    }

    fn sanitize(mut self) -> Self {
        if !self.eula {
            self.tos = false;
            self.pp = false;
        }
        self
    }

    fn to_string(self) -> String {
        let current_secs = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        format!(
            "\"EULA=true\" indicates that you read and accepted the End User Licence agreement. You can find our EULA at https://docs.tensamin.net/legal/eula/\
            \nEULA={}\
            \n\"PrivacyPolicy=true\" indicates that you read and accepted the Privacy Policy. You can find our Privacy Policy at https://docs.tensamin.net/legal/privacy-policy/\
            \nPrivacyPolicy={}\
            \n\"ToS=true\" indicates that you read and accepted the Terms of Service. You can find our Terms of Service at https://docs.tensamin.net/legal/terms-of-service/\
            \nToS={}\
            \nThis file reflects the current consent state used by the application.\
            \nIt may be regenerated or overwritten by the application.\
            \nThis file was last edited by Tensamin at:\
            \nUNIX-SECOND={}",
            self.eula, self.pp, self.tos, current_secs
        )
    }

    fn from_str(s: &str) -> Self {
        let mut eula = false;
        let mut pp = false;
        let mut tos = false;

        for line in s.lines() {
            if let Some(v) = line.strip_prefix("EULA=") {
                eula = v == "true";
            } else if let Some(v) = line.strip_prefix("ToS=") {
                tos = v == "true";
            } else if let Some(v) = line.strip_prefix("PrivacyPolicy=") {
                pp = v == "true";
            }
        }

        Self {
            eula,
            pp,
            tos,
            focus: Focus::Cancel,
        }
        .sanitize()
    }
    fn can_continue(&self) -> bool {
        self.eula
    }
    fn can_continue_all(&self) -> bool {
        self.eula && self.pp && self.tos
    }

    fn next(&mut self) {
        self.focus = match self.focus {
            Focus::Eula => Focus::Tos,
            Focus::Tos => Focus::Pp,
            Focus::Pp => Focus::Cancel,
            Focus::Cancel => {
                if self.can_continue() {
                    Focus::Continue
                } else {
                    Focus::Eula
                }
            }
            Focus::Continue => Focus::ContinueAll,
            Focus::ContinueAll => Focus::Eula,
        };
    }

    fn prev(&mut self) {
        self.focus = match self.focus {
            Focus::Eula => Focus::ContinueAll,
            Focus::Tos => Focus::Eula,
            Focus::Pp => Focus::Tos,
            Focus::Cancel => Focus::Pp,
            Focus::Continue => Focus::Cancel,
            Focus::ContinueAll => Focus::Continue,
        };
    }
}

fn run_consent_ui() -> UserChoice {
    let mut terminal = ratatui::init();

    let mut state = ConsentUiState {
        eula: false,
        tos: false,
        pp: false,
        focus: Focus::Eula,
    };

    let result = loop {
        terminal
            .draw(|f| {
                let mut needed_height = 5;

                let size = f.area();

                if size.height < 6
                || size.width < 27 {
                    f.render_widget(Line::from(format!("too small {}/63 by {}/12", size.width, size.height)), size);
                    return;
                }

                let max_width = 132;
                let max_height = 20;

                let content_width = if max_width < size.width { max_width } else { size.width} ;
                let content_height = if max_height < size.height { max_height } else { size.height} ;

                let horizontal_margin = (size.width.saturating_sub(content_width)) / 2;
                let vertical_margin = (size.height.saturating_sub(content_height)) / 2;

                let chunks = Layout::default()
                    .direction(Direction::Vertical)
                    .constraints([Constraint::Min(7), Constraint::Length(3)])
                    .split(Rect {
                        x: horizontal_margin,
                        y: vertical_margin,
                        width: content_width,
                        height: content_height,
                    });
                let eula_text = if size.width < 76 {
                    "EULA ¹ (https://docs.tensamin.net/legal/eula/)"
                } else {
                    "End User Licence Agreement ¹ (https://docs.tensamin.net/legal/eula/)"
                };
                let tos_text = if size.width < 76 {
                    "ToS ² (https://docs.tensamin.net/legal/terms-of-service/)"
                } else {
                    "Terms of Service ² (https://docs.tensamin.net/legal/terms-of-service/)"
                };
                let pp_text = if size.width < 76 {
                    "PP ² (https://docs.tensamin.net/legal/privacy-policy/)"
                } else {
                    "Privacy Policy ² (https://docs.tensamin.net/legal/privacy-policy/)"
                };

                let (mut optional_lines, agree_lines): (Vec<i16>, Vec<&str>) =
                    if size.width > 132 {
                        (
                            vec![7, 3, 4],
                            vec![
                                "",
                                "By selecting Continue, you confirm that you have read and agree to the End User License Agreement and applicable Terms of Service.",
                                "",
                                "Tensamin services require acceptance of the Terms of Service and Privacy Policy.",
                            ]
                        )
                    } else  if size.width > 68 {
                        (
                            vec![8, 3, 4],
                            vec![
                                "",
                                "By selecting Continue, you confirm that you have read and agree",
                                "to the End User License Agreement and applicable Terms of Service.",
                                "",
                                "Tensamin services require acceptance of the ToS and Privacy Policy.",
                            ]
                        )
                    } else {
                        (
                            vec![9, 3, 4],
                            vec![
                                "",
                                "By selecting Continue, you confirm that you have",
                                "read and agree to the End User License Agreement",
                                "and applicable Terms of Service.",
                                "",
                                "Tensamin services require acceptance of the",
                                "Terms of Service and Privacy Policy."
                            ]
                        )
                    };
                let mut text_lines = vec![
                    checkbox(eula_text, state.eula, state.focus == Focus::Eula, true),
                    checkbox(tos_text, state.tos, state.focus == Focus::Tos, state.eula),
                    checkbox(pp_text, state.pp, state.focus == Focus::Pp, state.eula),
                    Line::from(""),
                    Line::from("¹ Necessary, ² Optional"),
                ];

                for line in agree_lines {
                    text_lines.insert(text_lines.len(), Line::from(line));
                }

                while size.height - 5 < text_lines.len() as u16 {
                    if optional_lines.len() == 0 {
                        break;
                    }
                    text_lines.remove(optional_lines[0] as usize);
                    optional_lines.remove(0);
                }
                needed_height += text_lines.len();



                if size.width < 63 || size.height < needed_height as u16 {
                    let width_style = if size.width > 76 {
                        Style::default().fg(Color::Green)
                    } else if size.width >= 63 {
                        Style::default().fg(Color::Yellow)
                    } else {
                        Style::default().fg(Color::Red)
                    };

                    let height_style = if size.height < needed_height as u16 {
                        Style::default().fg(Color::Red)
                    } else if size.height < 17 {
                        Style::default().fg(Color::Yellow)
                    } else {
                        Style::default().fg(Color::Green)
                    };

                    let warning_text = Text::from(vec![
                        Line::from(vec![
                            Span::raw("Width: "),
                            Span::styled(format!("{}", size.width), width_style),
                            Span::raw(" / 63"),
                        ]),
                        Line::from(vec![
                            Span::raw("Height: "),
                            Span::styled(format!("{}", size.height), height_style),
                            Span::raw(format!(" / {}", needed_height)),
                        ]),
                    ]);

                    let warning = Paragraph::new(warning_text)
                        .alignment(Alignment::Center)
                        .block(
                            Block::default()
                                .borders(Borders::ALL)
                                .title("UI Too Small (Q to Quit)"),
                        );

                    f.render_widget(warning, size);
                    return;
                }

                let consent_block = Paragraph::new(Text::from(text_lines))
                    .block(Block::default().title(" Tensamin User Consent ").borders(Borders::ALL));
                f.render_widget(consent_block, chunks[0]);

                draw_buttons(f, chunks[1], &state);
            })
            .unwrap();

        if event::poll(Duration::from_millis(200)).unwrap() {
            if let Event::Key(key) = event::read().unwrap() {
                match key.code {
                    KeyCode::Esc => break UserChoice::Deny,
                    KeyCode::Up => state.prev(),
                    KeyCode::Down | KeyCode::Tab => state.next(),
                    KeyCode::Char('q') | KeyCode::Char('Q') => break UserChoice::Deny,
                    KeyCode::Char(' ') => match state.focus {
                        Focus::Eula => {
                            state.eula = !state.eula;
                            state.tos = false;
                            state.pp = false;
                        }
                        Focus::Tos => {
                            if state.eula {
                                state.tos = !state.tos
                            }
                        }
                        Focus::Pp => {
                            if state.eula {
                                state.pp = !state.pp
                            }
                        }
                        _ => {}
                    },
                    KeyCode::Enter => match state.focus {
                        Focus::Cancel => break UserChoice::Deny,
                        Focus::Continue if state.can_continue() => break UserChoice::AcceptEULA,
                        Focus::ContinueAll if state.can_continue_all() => {
                            break UserChoice::AcceptAll;
                        }
                        _ => {}
                    },
                    _ => {}
                }
            }
        }
    };
    ratatui::restore();

    result
}

#[allow(mismatched_lifetime_syntaxes)]
fn checkbox(label: &str, checked: bool, active: bool, allowed: bool) -> Line {
    let box_char = if checked { "[x]" } else { "[ ]" };
    let (box_style, text_style) = if active {
        if allowed {
            (
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            )
        } else {
            (
                Style::default().fg(Color::Gray),
                Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
            )
        }
    } else {
        (Style::default(), Style::default())
    };
    Line::from(vec![
        Span::styled(box_char, box_style),
        Span::raw(" "),
        Span::styled(label, text_style),
    ])
}

fn draw_button(f: &mut ratatui::Frame, area: Rect, label: &str, style: Style) {
    let p = Paragraph::new(Span::styled(label, style))
        .alignment(Alignment::Center)
        .block(Block::default().borders(Borders::ALL));
    f.render_widget(p, area);
}
fn draw_buttons(f: &mut ratatui::Frame, area: Rect, state: &ConsentUiState) {
    let buttons = vec![
        ("[Q] Cancel", Focus::Cancel),
        ("Continue", Focus::Continue),
        ("Continue with Tensamin Services", Focus::ContinueAll),
    ];

    let padding = 2;
    let min_widths: Vec<u16> = buttons
        .iter()
        .map(|(label, _)| label.len() as u16 + padding)
        .collect();

    let widths = compute_widths(area.width, &min_widths);

    let mut x = area.x;

    for ((label, focus), width) in buttons.iter().zip(widths) {
        let chunk = Rect {
            x,
            y: area.y,
            width,
            height: area.height,
        };
        x += width;

        let style = match focus {
            Focus::Cancel => {
                if state.focus == Focus::Cancel {
                    Style::default()
                        .fg(Color::Black)
                        .bg(Color::Red)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(Color::Red)
                }
            }
            Focus::Continue => {
                if state.focus == Focus::Continue && state.can_continue() {
                    Style::default()
                        .fg(Color::Black)
                        .bg(Color::Green)
                        .add_modifier(Modifier::BOLD)
                } else if state.can_continue() {
                    Style::default().fg(Color::Green)
                } else {
                    Style::default().fg(Color::DarkGray)
                }
            }
            Focus::ContinueAll => {
                if state.focus == Focus::ContinueAll && state.can_continue_all() {
                    Style::default()
                        .fg(Color::Black)
                        .bg(Color::Green)
                        .add_modifier(Modifier::BOLD)
                } else if state.can_continue() && state.pp {
                    Style::default().fg(Color::Green)
                } else {
                    Style::default().fg(Color::DarkGray)
                }
            }
            _ => Style::default().fg(Color::DarkGray),
        };

        draw_button(f, chunk, label, style);
    }
}
fn compute_widths(area_width: u16, min_widths: &[u16]) -> Vec<u16> {
    let mut widths = vec![0; min_widths.len()];
    let mut remaining: Vec<usize> = (0..min_widths.len()).collect();

    let mut remaining_width = area_width;

    while !remaining.is_empty() {
        let count = remaining.len() as u16;
        let equal = remaining_width / count;

        let mut clamped = Vec::new();

        for &i in &remaining {
            if min_widths[i] > equal {
                widths[i] = min_widths[i];
                remaining_width -= min_widths[i];
                clamped.push(i);
            }
        }

        if clamped.is_empty() {
            let mut remainder = remaining_width % count;
            for &i in &remaining {
                widths[i] = equal
                    + if remainder > 0 {
                        remainder -= 1;
                        1
                    } else {
                        0
                    };
            }
            break;
        }

        remaining.retain(|i| !clamped.contains(i));
    }

    widths
}
