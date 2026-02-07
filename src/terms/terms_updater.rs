use crate::terms::{
    buttons::{checkbox, draw_buttons},
    consent_state::{ConsentState, UpdateDecision, UserChoice},
    doc::Doc,
    focus::Focus,
    md_viewer::FileViewer,
    terms_getter::{Type, get_newest_link, get_terms},
};
use chrono::{Local, TimeZone, Utc};
use crossterm::event::{self, Event, KeyCode};
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, Paragraph},
};
use std::time::Duration;
pub async fn run_consent_ui(
    mut consent: ConsentState,
    consent_eula: UpdateDecision,
    consent_tos: UpdateDecision,
    consent_pp: UpdateDecision,
) -> ConsentState {
    let mut terminal = ratatui::init();

    let (eula_needed, eula_future, eula_for_future): (bool, bool, Option<Doc>) = match consent_eula
    {
        UpdateDecision::NoChange => (false, false, None),
        UpdateDecision::Future { newest } => (true, true, Some(newest)),
        UpdateDecision::Forced(doc) => (true, false, Some(doc)),
    };
    let (tos_needed, tos_future, tos_for_future): (bool, bool, Option<Doc>) = match consent_tos {
        UpdateDecision::NoChange => (false, false, None),
        UpdateDecision::Future { newest } => (true, true, Some(newest)),
        UpdateDecision::Forced(doc) => (true, false, Some(doc)),
    };
    let (pp_needed, pp_future, pp_for_future): (bool, bool, Option<Doc>) = match consent_pp {
        UpdateDecision::NoChange => (false, false, None),
        UpdateDecision::Future { newest } => (true, true, Some(newest)),
        UpdateDecision::Forced(doc) => (true, false, Some(doc)),
    };
    let update_needed =
        (eula_needed && !eula_future) || (tos_needed && !tos_future) || (pp_needed && !pp_future);
    let (mut eula, mut tos, mut pp) = (!eula_needed, !tos_needed, !pp_needed);
    let mut focus = if eula_needed {
        Focus::Eula
    } else if tos_needed {
        Focus::Tos
    } else if pp_needed {
        Focus::Pp
    } else {
        Focus::Cancel
    };

    let result = loop {
        let mut too_small = false;
        terminal
            .draw(|f| {
                let mut needed_height = 5;

                let size = f.area();

                if size.height < 6
                || size.width < 27 {
                    f.render_widget(Line::from(format!("too small {}/63 by {}/15", size.width, size.height)), size);
                    return;
                }

                let max_width = 150;
                let max_height = 30;

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


                let mut text_lines: Vec<Line> = Vec::new();
                let mut header_lines = 0;
                let seperator = if size.width > 72 {
                    header_lines += 3;
                    text_lines.push(Line::from("You previously accepted earlier versions of Tensamin’s legal documents."));
                    text_lines.push(Line::from("Some of them have been updated and are listed below for your review."));
                    text_lines.push(Line::from(""));
                    2
                } else {
                    header_lines += 5;
                    text_lines.push(Line::from("You previously accepted earlier "));
                    text_lines.push(Line::from("versions of Tensamin’s legal documents."));
                    text_lines.push(Line::from("Some of them have been updated and"));
                    text_lines.push(Line::from("are listed below for your review."));
                    text_lines.push(Line::from(""));
                    4
                };

                if eula_needed {
                    header_lines += 1;
                    if eula_future {
                        if size.width < 80 {
                            text_lines.push(checkbox("EULA ¹³ (https://legal.tensamin.net/eula/newest/)", eula, focus == Focus::Eula, true));

                            header_lines += 1;
                            let unix_timestamp = eula_for_future.clone().unwrap().get_time() as i64;
                            let datetime = Utc.timestamp_opt(unix_timestamp, 0).single().unwrap();
                            let date = datetime.with_timezone(&Local).format("%Y-%m-%d at %H:%M");
                            text_lines.push(Line::from(format!("    Goes into effect on {}", date)));
                        } else {
                            text_lines.push(checkbox("End User Licence Agreement ¹³ (https://legal.tensamin.net/eula/newest/)", eula, focus == Focus::Eula, true));

                            header_lines += 1;
                            let unix_timestamp = eula_for_future.clone().unwrap().get_time() as i64;
                            let datetime = Utc.timestamp_opt(unix_timestamp, 0).single().unwrap();
                            let date = datetime.with_timezone(&Local).format("%Y-%m-%d at %H:%M");
                            text_lines.push(Line::from(format!("    Goes into effect on {}", date)));
                        }
                    } else {
                        if size.width < 80 {
                            text_lines.push(checkbox("EULA ¹ (https://legal.tensamin.net/eula/newest/)", eula, focus == Focus::Eula, true));
                        } else {
                            text_lines.push(checkbox("End User Licence Agreement ¹ (https://legal.tensamin.net/eula/newest/)", eula, focus == Focus::Eula, true));
                        }
                    }
                }

                if tos_needed {
                    header_lines += 1;
                    if tos_future {
                        if size.width < 80 {
                            text_lines.push(checkbox("ToS ²³ (https://legal.tensamin.net/tos/newest/)", tos, focus == Focus::Tos, eula));

                            header_lines += 1;
                            let unix_timestamp = tos_for_future.clone().unwrap().get_time() as i64;
                            let datetime = Utc.timestamp_opt(unix_timestamp, 0).single().unwrap();
                            let date = datetime.with_timezone(&Local).format("%Y-%m-%d at %H:%M");
                            text_lines.push(Line::from(format!("    Goes into effect on {}", date)));
                        } else {
                            text_lines.push(checkbox("Terms of Service ²³ (https://legal.tensamin.net/terms-of-service/newest/)", tos, focus == Focus::Tos, eula));

                            header_lines += 1;
                            let unix_timestamp = tos_for_future.clone().unwrap().get_time() as i64;
                            let datetime = Utc.timestamp_opt(unix_timestamp, 0).single().unwrap();
                            let date = datetime.with_timezone(&Local).format("%Y-%m-%d at %H:%M");
                            text_lines.push(Line::from(format!("    Goes into effect on {}", date)));
                        }
                    } else {
                        if size.width < 80 {
                            text_lines.push(checkbox("ToS ² (https://legal.tensamin.net/tos/newest/)", tos, focus == Focus::Tos, eula));
                        } else {
                            text_lines.push(checkbox("Terms of Service ² (https://legal.tensamin.net/terms-of-service/newest/)", tos, focus == Focus::Tos, eula));
                        }
                    }
                }

                if pp_needed {
                    header_lines += 1;
                    if pp_future {
                        if size.width < 80 {
                            text_lines.push(checkbox("PP ²³ (https://legal.tensamin.net/privacy-policy/newest/)", pp, focus == Focus::Pp, eula));

                            header_lines += 1;
                            let unix_timestamp = pp_for_future.clone().unwrap().get_time() as i64;
                            let datetime = Utc.timestamp_opt(unix_timestamp, 0).single().unwrap();
                            let date = datetime.with_timezone(&Local).format("%Y-%m-%d at %H:%M");
                            text_lines.push(Line::from(format!("    Goes into effect on {}", date)));
                        } else {
                            text_lines.push(checkbox("Privacy Policy ²³ (https://legal.tensamin.net/privacy-policy/newest/)", pp, focus == Focus::Pp, eula));

                            header_lines += 1;
                            let unix_timestamp = pp_for_future.clone().unwrap().get_time() as i64;
                            let datetime = Utc.timestamp_opt(unix_timestamp, 0).single().unwrap();
                            let date = datetime.with_timezone(&Local).format("%Y-%m-%d at %H:%M");
                            text_lines.push(Line::from(format!("    Goes into effect on {}", date)));
                        }
                    } else {
                        if size.width < 80 {
                            text_lines.push(checkbox("PP ² (https://legal.tensamin.net/privacy-policy/newest/)", pp, focus == Focus::Pp, eula));
                        } else {
                            text_lines.push(checkbox("Privacy Policy ² (https://legal.tensamin.net/privacy-policy/newest/)", pp, focus == Focus::Pp, eula));
                        }
                    }
                }

                text_lines.push(Line::from(""));
                text_lines.push(Line::from("¹ Necessary to run the program"));
                text_lines.push(Line::from("² Optional - only for Tensamin services"));
                if size.width < 100 {
                    text_lines.push(Line::from("³ Future version - consent stored now, takes effect later"));
                } else {
                    text_lines.push(Line::from("³ Future version - You’ll continue using this version, automatically updated when changes apply."));
                }

                text_lines.push(Line::from(""));

                let mut optional_lines: Vec<i16> =
                    if size.width > 143 {
                        if tos_needed || pp_needed {
                            text_lines.push(Line::from("By selecting Downgrade, you confirm that you have agree to the End User License Agreement and applicable Terms of Service."));
                            text_lines.push(Line::from("On Downgrade: Tensamin Services will deactivate once these changes apply."));
                        } else {
                            text_lines.push(Line::from("By selecting Continue, you confirm that you have agree to the End User License Agreement and applicable Terms of Service."));
                        }
                        text_lines.push(Line::from(""));
                        text_lines.push(Line::from("Tensamin services require acceptance of the Terms of Service and Privacy Policy."));
                        text_lines.push(Line::from(""));
                        text_lines.push(Line::from("While having a document selected press O to view in this UI or press L to open as a link."));

                        if tos_needed || pp_needed {
                            vec![header_lines + 7, header_lines, header_lines + 7, seperator, header_lines + 2]
                        } else {
                            vec![header_lines + 6, header_lines, header_lines + 6, seperator, header_lines + 2]
                        }
                    } else if size.width > 92 {
                        if tos_needed || pp_needed {
                            text_lines.push(Line::from("By selecting Continue, you confirm that you have agree to the End User"));
                            text_lines.push(Line::from("License Agreement and applicable Terms of Service."));
                            text_lines.push(Line::from("On Downgrade: Tensamin Services will deactivate once these changes apply."));
                        } else {
                            text_lines.push(Line::from("By selecting Continue, you confirm that you have agree to the End User"));
                            text_lines.push(Line::from("License Agreement and applicable Terms of Service."));
                        }

                        text_lines.push(Line::from(""));
                        text_lines.push(Line::from("Tensamin services require acceptance of the Terms of Service and Privacy Policy."));
                        text_lines.push(Line::from(""));
                        text_lines.push(Line::from("While having a document selected press O to view in this UI or press L to open as a link."));

                        if tos_needed || pp_needed {
                            vec![header_lines + 8, header_lines, header_lines + 8, seperator, header_lines + 2]
                        } else {
                            vec![header_lines + 7, header_lines, header_lines + 7, seperator, header_lines + 2]
                        }
                    } else if size.width > 73 {
                        if tos_needed || pp_needed {
                            text_lines.push(Line::from("By selecting Continue, you confirm that you have read, understood and"));
                            text_lines.push(Line::from("agree to the End User License Agreement and applicable Terms of Service."));
                            text_lines.push(Line::from("On Downgrade: Tensamin Services will deactivate once these changes apply."));
                        } else {
                            text_lines.push(Line::from("By selecting Continue, you confirm that you have read, understood and"));
                            text_lines.push(Line::from("agree to the End User License Agreement and applicable Terms of Service."));
                        }
                        text_lines.push(Line::from("",));
                        text_lines.push(Line::from("Tensamin services require acceptance of the ToS and Privacy Policy.",));
                        text_lines.push(Line::from("",));
                        text_lines.push(Line::from("While having a document selected press O to view in this UI or press L",));
                        text_lines.push(Line::from("to open as a link.",));

                        if tos_needed || pp_needed {
                            vec![header_lines + 8, header_lines, header_lines + 8, seperator, header_lines + 2]
                        } else {
                            vec![header_lines + 7, header_lines, header_lines + 7, seperator, header_lines + 2]
                        }
                    } else {
                        if tos_needed || pp_needed {
                            text_lines.push(Line::from("By selecting Continue, you confirm that you have",));
                            text_lines.push(Line::from("agree to the End User License",));
                            text_lines.push(Line::from("Agreement and applicable Terms of Service.",));
                            text_lines.push(Line::from("On Downgrade: Tensamin Services will deactivate"));
                            text_lines.push(Line::from("once these changes apply."));
                        } else {
                            text_lines.push(Line::from("By selecting Continue, you confirm that you have",));
                            text_lines.push(Line::from("agree to the End User License",));
                            text_lines.push(Line::from("Agreement and applicable Terms of Service.",));
                        }
                        text_lines.push(Line::from("",));
                        text_lines.push(Line::from("Tensamin services require acceptance of the",));
                        text_lines.push(Line::from("Terms of Service and Privacy Policy.",));
                        text_lines.push(Line::from("",));
                        text_lines.push(Line::from("While having a document selected press O to view",));
                        text_lines.push(Line::from("in this UI or press L to open as a link.",));
                        if tos_needed || pp_needed {
                            vec![header_lines + 10, header_lines, header_lines + 11, seperator, header_lines + 2]
                        } else {
                            vec![header_lines + 8, header_lines, header_lines + 9, seperator, header_lines + 2]
                        }
                    };


                while size.height - 5 < text_lines.len() as u16 {
                    if optional_lines.len() == 0 {
                        break;
                    }
                    text_lines.remove(optional_lines[0] as usize);
                    optional_lines.remove(0);
                }
                needed_height += text_lines.len();


                let q_informer = if update_needed {
                    "Q to Exit"
                } else {
                    "Q to Cancel"
                };
                if size.width < 60 || size.height < needed_height as u16 {
                    let width_style = if size.width > 76 {
                        Style::default().fg(Color::Green)
                    } else if size.width >= 60 {
                        Style::default().fg(Color::Yellow)
                    } else {
                        Style::default().fg(Color::Red)
                    };

                    let height_style = if size.height > 20 {
                        Style::default().fg(Color::Green)
                    } else if size.height >= (header_lines as u16 + 10) {
                        Style::default().fg(Color::Yellow)
                    } else {
                        Style::default().fg(Color::Red)
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
                            Span::raw(format!(" / {}", header_lines + 10)),
                        ]),
                    ]);


                    let warning = Paragraph::new(warning_text)
                        .alignment(Alignment::Center)
                        .block(
                            Block::default()
                                .borders(Borders::ALL)
                                .title(format!("UI Too Small [{}]", q_informer)),
                        );

                    too_small = true;
                    f.render_widget(warning, size);
                    return;
                }

                let consent_block = Paragraph::new(Text::from(text_lines))
                    .block(Block::default().title(format!(" Update Tensamin User Consent [{}] ", q_informer)).borders(Borders::ALL));
                f.render_widget(consent_block, chunks[0]);

                let downgrade_scenario = tos_needed || pp_needed;
                draw_buttons(
                    f,
                    chunks[1],
                    focus,
                    (eula, tos && pp),
                    update_needed,
                    downgrade_scenario,
                    pp_needed || tos_needed
                );
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
                let mut possible_states = vec![Focus::Eula];

                if tos_needed {
                    possible_states.push(Focus::Tos);
                }
                if pp_needed {
                    possible_states.push(Focus::Pp);
                }

                possible_states.push(Focus::Cancel);
                if eula {
                    possible_states.push(Focus::Continue);
                    if tos && pp && (pp_needed || tos_needed) {
                        possible_states.push(Focus::ContinueAll);
                    }
                }
                match key.code {
                    KeyCode::Esc => break UserChoice::Deny,
                    KeyCode::Up | KeyCode::Left => focus.prev(&possible_states),
                    KeyCode::Down | KeyCode::Right | KeyCode::Tab => focus.next(&possible_states),
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

                        let _ = open::that(get_newest_link(terms_type));
                    }
                    KeyCode::Char(' ') | KeyCode::Enter => match focus {
                        Focus::Eula => {
                            eula = !eula;
                            tos = !tos_needed;
                            pp = !pp_needed;
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

    match result {
        UserChoice::AcceptAll => {
            consent.accepted_eula = true;
            consent.accepted_tos = true;
            consent.accepted_pp = true;
            if let Some(_) = eula_for_future {
                consent.future_eula = eula_for_future;
            }
            if let Some(_) = tos_for_future {
                consent.future_tos = tos_for_future;
            }
            if let Some(_) = pp_for_future {
                consent.future_privacy = pp_for_future;
            }
        }
        UserChoice::AcceptEULA => {
            consent.accepted_eula = true;
            if let Some(_) = eula_for_future {
                consent.future_eula = eula_for_future;
            }
        }
        UserChoice::Deny => {}
    }

    consent
}
