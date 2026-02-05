use crate::terms::{
    consent_state::UserChoice,
    focus::Focus,
    md_viewer::FileViewer,
    terms_getter::{Type, get_link, get_terms},
};
use crossterm::event::{self, Event, KeyCode};
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, Paragraph},
};
use std::time::Duration;

pub async fn run_consent_ui() -> UserChoice {
    let mut terminal = ratatui::init();

    let (mut eula, mut tos, mut pp) = (false, false, false);
    let mut focus = Focus::Eula;

    let result = loop {
        let mut too_small = false;
        terminal
            .draw(|f| {
                let mut needed_height = 5;

                let size = f.area();

                if size.height < 6
                || size.width < 27 {
                    f.render_widget(Line::from(format!("too small {}/63 by {}/12", size.width, size.height)), size);
                    return;
                }

                let max_width = 150;
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
                let eula_text = if size.width < 70 {
                    "EULA ¹ (https://legal.tensamin.net/eula/)"
                } else {
                    "End User Licence Agreement ¹ (https://legal.tensamin.net/eula/)"
                };
                let tos_text = if size.width < 72 {
                    "ToS ² (https://legal.tensamin.net/terms-of-service/)"
                } else {
                    "Terms of Service ² (https://legal.tensamin.net/terms-of-service/)"
                };
                let pp_text = if size.width < 68 {
                    "PP ² (https://legal.tensamin.net/privacy-policy/)"
                } else {
                    "Privacy Policy ² (https://legal.tensamin.net/privacy-policy/)"
                };

                let (mut optional_lines, agree_lines): (Vec<i16>, Vec<&str>) =
                    if size.width > 143 {
                        (
                            vec![7, 3, 7, 4],
                            vec![
                                "",
                                "By selecting Continue, you confirm that you have read, understood and agree to the End User License Agreement and applicable Terms of Service.",
                                "",
                                "Tensamin services require acceptance of the Terms of Service and Privacy Policy.",
                                "",
                                "While having a document selected press O to view in this UI or press L to open as a link.",
                            ]
                        )
                    } else if size.width > 92 {
                        (
                            vec![8, 3, 8, 4],
                            vec![
                                "",
                                "By selecting Continue, you confirm that you have read, understood and agree to the End User",
                                "License Agreement and applicable Terms of Service.",
                                "",
                                "Tensamin services require acceptance of the Terms of Service and Privacy Policy.",
                                "",
                                "While having a document selected press O to view in this UI or press L to open as a link.",
                            ]
                        )
                    } else if size.width > 73 {
                        (
                            vec![8, 3, 8, 4],
                            vec![
                                "",
                                "By selecting Continue, you confirm that you have read, understood and",
                                "agree to the End User License Agreement and applicable Terms of Service.",
                                "",
                                "Tensamin services require acceptance of the ToS and Privacy Policy.",
                                "",
                                "While having a document selected press O to view in this UI or press L",
                                "to open as a link.",
                            ]
                        )
                    } else {
                        (
                            vec![9, 3, 10, 4],
                            vec![
                                "",
                                "By selecting Continue, you confirm that you have",
                                "read, understood and agree to the End User License",
                                "Agreement and applicable Terms of Service.",
                                "",
                                "Tensamin services require acceptance of the",
                                "Terms of Service and Privacy Policy.",
                                "",
                                "While having a document selected press O to view",
                                "in this UI or press L to open as a link.",
                            ]
                        )
                    };
                let mut text_lines = vec![
                    checkbox(eula_text, eula, focus == Focus::Eula, true),
                    checkbox(tos_text, tos, focus == Focus::Tos, eula),
                    checkbox(pp_text, pp, focus == Focus::Pp, eula),
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



                if size.width < 60 || size.height < needed_height as u16 {
                    let width_style = if size.width > 76 {
                        Style::default().fg(Color::Green)
                    } else if size.width >= 60 {
                        Style::default().fg(Color::Yellow)
                    } else {
                        Style::default().fg(Color::Red)
                    };

                    let height_style = if size.height < 12 {
                        Style::default().fg(Color::Red)
                    } else if size.height > 19 {
                        Style::default().fg(Color::Green)
                    } else {
                        Style::default().fg(Color::Yellow)
                    };

                    let warning_text = Text::from(vec![
                        Line::from(vec![
                            Span::raw("Width: "),
                            Span::styled(format!("{}", size.width), width_style),
                            Span::raw(" / 60"),
                        ]),
                        Line::from(vec![
                            Span::raw("Height: "),
                            Span::styled(format!("{}", size.height), height_style),
                            Span::raw(format!(" / 12")),
                        ]),
                    ]);

                    let warning = Paragraph::new(warning_text)
                        .alignment(Alignment::Center)
                        .block(
                            Block::default()
                                .borders(Borders::ALL)
                                .title("UI Too Small (Q to Quit)"),
                        );

                    too_small = true;
                    f.render_widget(warning, size);
                    return;
                }

                let consent_block = Paragraph::new(Text::from(text_lines))
                    .block(Block::default().title(" Tensamin User Consent [Q to Quit] ").borders(Borders::ALL));
                f.render_widget(consent_block, chunks[0]);

                draw_buttons(f, chunks[1], (eula, tos, pp));
            })
            .unwrap();

        if event::poll(Duration::from_millis(200)).unwrap() {
            if let Event::Key(key) = event::read().unwrap() {
                if too_small {
                    if matches!(key.code, KeyCode::Char('q') | KeyCode::Char('Q')) {
                        break UserChoice::Deny;
                    } else {
                        continue;
                    }
                }
                match key.code {
                    KeyCode::Esc => break UserChoice::Deny,
                    KeyCode::Up | KeyCode::Left => focus.prev((eula, tos, pp)),
                    KeyCode::Down | KeyCode::Right | KeyCode::Tab => focus.next((eula, tos, pp)),
                    KeyCode::Char('q') | KeyCode::Char('Q') => break UserChoice::Deny,
                    KeyCode::Char('o') | KeyCode::Char('O') => {
                        let terms_type = match focus {
                            Focus::Eula => Type::EULA,
                            Focus::Tos => Type::TOS,
                            Focus::Pp => Type::PP,
                            _ => continue,
                        };

                        if let Some(eula) = get_terms(terms_type.clone()).await {
                            terminal = FileViewer::new(terms_type.to_string(), &eula)
                                .force_popup(terminal);
                        } else {
                            terminal = FileViewer::new(
                                terms_type.to_string(),
                                "### A loading error occured\
                                \nTry reloading this site or retry in a moment.",
                            )
                            .force_popup(terminal);
                        }
                    }
                    KeyCode::Char('l') | KeyCode::Char('L') => {
                        let terms_type = match focus {
                            Focus::Eula => Type::EULA,
                            Focus::Tos => Type::TOS,
                            Focus::Pp => Type::PP,
                            _ => continue,
                        };

                        let _ = open::that(get_link(terms_type));
                    }
                    KeyCode::Char(' ') => match focus {
                        Focus::Eula => {
                            eula = !eula;
                            tos = false;
                            pp = false;
                        }
                        Focus::Tos => {
                            if eula {
                                tos = !tos
                            }
                        }
                        Focus::Pp => {
                            if eula {
                                pp = !pp
                            }
                        }
                        _ => {}
                    },
                    KeyCode::Enter => match focus {
                        Focus::Cancel => break UserChoice::Deny,
                        Focus::Continue if eula => break UserChoice::AcceptEULA,
                        Focus::ContinueAll if eula && tos && pp => {
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
fn draw_buttons(f: &mut ratatui::Frame, area: Rect, state: (bool, bool, bool)) {
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
                if focus == &Focus::Cancel {
                    Style::default()
                        .fg(Color::Black)
                        .bg(Color::Red)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(Color::Red)
                }
            }
            Focus::Continue => {
                if focus == &Focus::Continue && state.0 {
                    Style::default()
                        .fg(Color::Black)
                        .bg(Color::Green)
                        .add_modifier(Modifier::BOLD)
                } else if state.0 && state.1 && state.2 {
                    Style::default().fg(Color::Green)
                } else {
                    Style::default().fg(Color::DarkGray)
                }
            }
            Focus::ContinueAll => {
                if focus == &Focus::ContinueAll && state.0 && state.1 && state.2 {
                    Style::default()
                        .fg(Color::Black)
                        .bg(Color::Green)
                        .add_modifier(Modifier::BOLD)
                } else if state.0 && state.1 && state.2 {
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
