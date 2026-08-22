// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! Turning a plan into prose, without letting the prose say more than the plan.
//!
//! ADR-0031 puts a `ResponsePlan` between typed Mind state and anything a person reads. The
//! realizer is what makes that boundary worth having: it may vary wording, order and language, and
//! it has no access to anything except the plan it was handed, so a fluent sentence cannot quietly
//! acquire a claim Mind never made.

use cybou_protocol::meaning::{Qualification, ResponsePlan};

/// A surface language a plan can be rendered in.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Language {
    /// English.
    English,
    /// Russian.
    Russian,
}

/// Render a plan as prose in one language.
///
/// The signature is the guarantee. There is no context parameter, no Journal handle and no
/// fallback text: everything this function can say, it was given. A renderer that could reach
/// further would be able to add an authoritative claim, and no amount of care in its wording would
/// make that safe.
#[must_use]
pub fn realize(plan: &ResponsePlan, language: Language) -> String {
    let mut lines = Vec::new();
    lines.push(match language {
        Language::English => opening_en(&plan.intent),
        Language::Russian => opening_ru(&plan.intent),
    });

    for point in &plan.key_points {
        lines.push(format!("- {point}"));
    }

    // Every qualification the plan carries reaches the reader. This is the half of C5 that only
    // matters if the renderer honours it: a plan that hedged and prose that did not would put the
    // confident reading in front of the person while the honest one stayed in a struct.
    for qualification in &plan.qualifications {
        lines.push(match language {
            Language::English => qualification_en(*qualification),
            Language::Russian => qualification_ru(*qualification),
        });
    }

    // What the plan rests on is said out loud rather than left implicit. A reader who wants to
    // check an answer has the contributions to check it against.
    if !plan.referenced_evidence.is_empty() {
        let ids = plan
            .referenced_evidence
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>()
            .join(", ");
        lines.push(match language {
            Language::English => format!("Based on: {ids}"),
            Language::Russian => format!("На основании: {ids}"),
        });
    }

    if plan.key_points.is_empty() && plan.qualifications.is_empty() {
        lines.push(match language {
            Language::English => "There is nothing established to report.".to_owned(),
            Language::Russian => "Установленного, о чём сообщить, нет.".to_owned(),
        });
    }

    lines.join("\n")
}

/// How a qualification reads in English.
///
/// Each says what is missing rather than softening the answer. "Some information may be missing" is
/// the phrasing this deliberately avoids: it hedges everything and names nothing, which leaves a
/// reader no better off than an unhedged claim would.
fn qualification_en(qualification: Qualification) -> String {
    match qualification {
        Qualification::NotRead => "Something this rests on was never read.".to_owned(),
        Qualification::Stale => {
            "This is older than its owner declared it stays good for.".to_owned()
        }
        Qualification::Partial => {
            "This was cut short by a limit, so it is not the whole of it.".to_owned()
        }
        Qualification::Withheld => "Something was held back from you.".to_owned(),
        Qualification::Unverified => {
            "The record behind this has not been verified to its end.".to_owned()
        }
    }
}

/// How a qualification reads in Russian.
fn qualification_ru(qualification: Qualification) -> String {
    match qualification {
        Qualification::NotRead => "Что-то, на чём это держится, не было прочитано.".to_owned(),
        Qualification::Stale => "Это старше срока, который назначил его владелец.".to_owned(),
        Qualification::Partial => "Ответ обрезан ограничением, это не всё.".to_owned(),
        Qualification::Withheld => "Часть была от вас удержана.".to_owned(),
        Qualification::Unverified => "Запись за этим не проверена до конца.".to_owned(),
    }
}

/// The English opening for a communicative intent.
///
/// An intent this renderer does not have a phrasing for is named as itself rather than dropped or
/// smoothed over: a plan that reached prose unrendered is a gap in the renderer, and the reader is
/// better served seeing it than seeing a sentence that pretends the intent was something else.
fn opening_en(intent: &str) -> String {
    match intent {
        "inform_status" => "Status:".to_owned(),
        "clarify_ambiguity" => "This is ambiguous, and needs one word from you:".to_owned(),
        "confirm_action" => "Confirm before this happens:".to_owned(),
        "explain_cause" => "What led to this:".to_owned(),
        other => format!("{other}:"),
    }
}

/// The Russian opening for a communicative intent.
fn opening_ru(intent: &str) -> String {
    match intent {
        "inform_status" => "Состояние:".to_owned(),
        "clarify_ambiguity" => "Здесь неоднозначность, нужно одно слово от вас:".to_owned(),
        "confirm_action" => "Подтвердите, прежде чем это произойдёт:".to_owned(),
        "explain_cause" => "Что к этому привело:".to_owned(),
        other => format!("{other}:"),
    }
}

#[cfg(test)]
mod tests {
    use uuid::Uuid;

    use super::*;

    #[test]
    fn a_hedge_in_the_plan_reaches_the_reader_in_both_languages() {
        // The renderer may choose wording, order and language. It may not choose to leave out that
        // the answer is qualified — that would put the confident reading in front of the person
        // while the honest one stayed in a struct.
        let mut hedged = plan();
        hedged.qualifications = vec![Qualification::Stale, Qualification::Withheld];

        let english = realize(&hedged, Language::English);
        let russian = realize(&hedged, Language::Russian);

        assert!(english.contains("older than"), "{english}");
        assert!(english.contains("held back"), "{english}");
        assert!(russian.contains("старше"), "{russian}");
        assert!(russian.contains("удержана"), "{russian}");
    }

    #[test]
    fn a_qualification_alone_is_not_nothing_to_report() {
        // "There is nothing established to report" beside a plan that says something was never read
        // would be the renderer contradicting the plan it was handed.
        let only_hedge = ResponsePlan {
            plan_id: Uuid::from_u128(3),
            intent: "inform_status".into(),
            key_points: Vec::new(),
            referenced_evidence: Vec::new(),
            qualifications: vec![Qualification::NotRead],
        };
        let rendered = realize(&only_hedge, Language::English);
        assert!(rendered.contains("never read"), "{rendered}");
        assert!(!rendered.contains("nothing established"), "{rendered}");
    }

    fn plan() -> ResponsePlan {
        ResponsePlan {
            plan_id: Uuid::from_u128(1),
            intent: "inform_status".into(),
            key_points: vec![
                "the journal chain is verified through its head".into(),
                "one obligation is open".into(),
            ],
            referenced_evidence: vec![Uuid::from_u128(42)],
            qualifications: Vec::new(),
        }
    }

    #[test]
    fn one_plan_renders_in_two_languages_without_changing_what_it_claims() {
        // C7 from the other direction: two realizations of one canonical object.
        let english = realize(&plan(), Language::English);
        let russian = realize(&plan(), Language::Russian);
        assert_ne!(english, russian);
        for rendered in [&english, &russian] {
            for point in &plan().key_points {
                assert!(
                    rendered.contains(point.as_str()),
                    "a realization must carry every claim the plan made"
                );
            }
        }
    }

    #[test]
    fn a_realization_says_nothing_the_plan_did_not() {
        // C6. The claim is structural — the renderer has no other input — and this holds it to it:
        // every line is either an opening for the stated intent, a key point, or the evidence.
        let rendered = realize(&plan(), Language::English);
        let allowed = {
            let mut allowed = vec![opening_en("inform_status")];
            allowed.extend(plan().key_points.iter().map(|point| format!("- {point}")));
            allowed.push(format!("Based on: {}", Uuid::from_u128(42)));
            allowed
        };
        for line in rendered.lines() {
            assert!(
                allowed.iter().any(|permitted| permitted == line),
                "the realization produced a line the plan did not supply: {line}"
            );
        }
    }

    #[test]
    fn a_plan_with_nothing_established_says_so_rather_than_filling_the_silence() {
        let empty = ResponsePlan {
            plan_id: Uuid::from_u128(2),
            intent: "inform_status".into(),
            key_points: Vec::new(),
            referenced_evidence: Vec::new(),
            qualifications: Vec::new(),
        };
        let rendered = realize(&empty, Language::English);
        assert!(rendered.contains("nothing established"));
    }

    #[test]
    fn an_intent_this_renderer_has_no_phrasing_for_is_named_rather_than_disguised() {
        let unusual = ResponsePlan {
            plan_id: Uuid::from_u128(3),
            intent: "propose_consolidation".into(),
            key_points: vec!["the chain has not been swept in nine hours".into()],
            referenced_evidence: Vec::new(),
            qualifications: Vec::new(),
        };
        let rendered = realize(&unusual, Language::English);
        assert!(rendered.starts_with("propose_consolidation:"));
    }
}
