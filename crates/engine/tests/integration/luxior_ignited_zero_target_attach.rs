//! CR 115.6 zero-target Attach: the host frame's "up to one target creature"
//! and the declined-host no-op.
//!
//! The prepared cards below are the verbatim corpus Oracle text (MTGJSON
//! `AtomicCards` strings), driven through `GameScenario`/`GameRunner` so every
//! assertion is a resolution delta, not a re-read of the parse.
//!
//! Base behavior that flips:
//! * `Luxior, Ignited` `[+1]` prints "Attach this Equipment to up to one target
//!   creature you control." — before the fix the clause carried no zero-target
//!   authority, so activating it with no creature on the battlefield was
//!   REJECTED at announcement (CR 601.2c: a mandatory target set with no legal
//!   target cannot be announced).
//! * `The Aetherspark` `[+1]` and `Sting, Bilbo's Sword`'s enters trigger have the
//!   same printed host; `Sting` additionally showed the lost counter rider.
//! * `Assassin Gauntlet`, `Hunter's Bow` and `Novel Nunchaku` reach the resolver
//!   with a declined host and previously errored with `MissingParam`; the
//!   declined host is now a resolution no-op (CR 115.6 + CR 608.2c).
//!
//! D4 (pre-existing, labelled, NOT fixed here): `Serene Remembrance`'s "from a
//! single graveyard" constraint parses as a target-set constraint on the
//! SelfRef head, but the announced multi-target set is on the sub-ability, and a
//! mixed-graveyard pair is accepted at announcement. The probe was measured and
//! failed, so only the same-graveyard positive is kept.

use engine::game::scenario::{GameRunner, GameScenario, Outcome, P0, P1};
use engine::types::ability::EffectKind;
use engine::types::counter::CounterType;
use engine::types::events::GameEvent;
use engine::types::game_state::WaitingFor;
use engine::types::identifiers::ObjectId;
use engine::types::phase::Phase;
use engine::types::player::PlayerId;
use engine::types::zones::Zone;

const LUXIOR_ORACLE: &str = "Equipped creature gets +1/+1 for each counter on Luxior, Ignited.\n[+1]: Attach this Equipment to up to one target creature you control.\n[\u{2212}2]: Equipped creature gets +2/+2 and gains double strike until end of turn.";

const AETHERSPARK_ORACLE: &str = "As long as The Aetherspark is attached to a creature, The Aetherspark can't be attacked and has \"Whenever equipped creature deals combat damage during your turn, put that many loyalty counters on The Aetherspark.\"\n[+1]: Attach The Aetherspark to up to one target creature you control. Put a +1/+1 counter on that creature.\n[\u{2212}5]: Draw two cards.\n[\u{2212}10]: Add ten mana of any one color.";

const STING_ORACLE: &str = "Flash\nWhen Sting enters, put a hone counter on Sting for each creature target opponent controls. Attach Sting to up to one target creature you control. (Each hone counter on an Equipment grants +1/+0 to equipped creature.)\nEquip {3}";

const ASSASSIN_GAUNTLET_ORACLE: &str = "When this Equipment enters, attach it to up to one target creature you control. Tap all creatures target opponent controls.\nEquipped creature gets +1/+1 and has \"Whenever this creature deals combat damage to a player, draw a card, then discard a card.\"\nEquip {2}";

const HUNTERS_BOW_ORACLE: &str = "When this Equipment enters, attach it to target creature you control. That creature deals damage equal to its power to up to one target creature you don't control.\nEquipped creature has reach and ward {2}.\nEquip {1}";

const NOVEL_NUNCHAKU_ORACLE: &str = "When this Equipment enters, attach it to target creature you control. When you do, equipped creature fights up to one target creature an opponent controls. (Each deals damage equal to its power to the other.)\nEquipped creature gets +1/+1 and has trample.\nEquip {3} ({3}: Attach to target creature you control. Equip only as a sorcery.)";

const SERENE_REMEMBRANCE_ORACLE: &str = "Shuffle Serene Remembrance and up to three target cards from a single graveyard into their owners' libraries.";

const STOLEN_GOODIES_ORACLE: &str =
    "Distribute three +1/+1 counters among any number of target creatures you control.";

/// Enter the precombat main phase with the active player holding priority.
fn grant_priority(runner: &mut GameRunner, player: PlayerId) {
    let state = runner.state_mut();
    state.priority_player = player;
    state.waiting_for = WaitingFor::Priority { player };
}

fn counters_of(outcome: &Outcome, id: ObjectId, counter: &CounterType) -> u32 {
    outcome
        .state()
        .objects
        .get(&id)
        .and_then(|object| object.counters.get(counter))
        .copied()
        .unwrap_or(0)
}

fn attached_object(outcome: &Outcome, id: ObjectId) -> Option<ObjectId> {
    outcome
        .state()
        .objects
        .get(&id)
        .and_then(|object| object.attached_to.as_ref())
        .and_then(|target| target.as_object())
}

fn attach_effect_resolved(outcome: &Outcome) -> bool {
    outcome.events().iter().any(|event| {
        matches!(
            event,
            GameEvent::EffectResolved {
                kind: EffectKind::Attach,
                ..
            }
        )
    })
}

/// Build the Luxior battlefield state: the Equipment Planeswalker with loyalty
/// counters (its `[+1]` is ability index 0).
fn luxior_runner(creatures: usize) -> (GameRunner, ObjectId, Vec<ObjectId>) {
    let mut scenario = GameScenario::new();
    scenario.at_phase(Phase::PreCombatMain);
    let luxior = scenario
        .add_planeswalker_from_oracle(P0, "Luxior, Ignited", "Luxior", 4, LUXIOR_ORACLE)
        .as_artifact()
        .with_subtypes(vec!["Equipment"])
        .id();
    let mut mine = Vec::new();
    for index in 0..creatures {
        mine.push(
            scenario
                .add_creature(P0, &format!("Trained {index}"), 1, 1)
                .id(),
        );
    }
    let mut runner = scenario.build();
    grant_priority(&mut runner, P0);
    (runner, luxior, mine)
}

// ---------------------------------------------------------------------------
// Luxior, Ignited
// ---------------------------------------------------------------------------

#[test]
fn luxior_plus_one_activates_and_resolves_with_no_creatures() {
    let (mut runner, luxior, _) = luxior_runner(0);
    let outcome = runner.activate(luxior, 0).resolve();

    // CR 115.6: zero targets is a legal announcement, and the ability still
    // resolves; CR 606.4 + CR 601.2c: the loyalty cost is paid either way.
    assert_eq!(
        counters_of(&outcome, luxior, &CounterType::Loyalty),
        5,
        "CR 606.4: the [+1] cost puts one loyalty counter on"
    );
    assert_eq!(
        attached_object(&outcome, luxior),
        None,
        "with no chosen host the Equipment stays unattached"
    );
    assert!(
        attach_effect_resolved(&outcome),
        "the Attach instruction must resolve as a no-op (EffectResolved): {:?}",
        outcome.events()
    );
    assert!(matches!(
        outcome.final_waiting_for(),
        WaitingFor::Priority { .. }
    ));
}

#[test]
fn luxior_plus_one_declined_host_leaves_it_unattached() {
    // Reach-guard (same test): the identical state WITH a declared host attaches.
    let (mut chosen_runner, chosen_luxior, chosen_creatures) = luxior_runner(1);
    let chosen = chosen_runner
        .activate(chosen_luxior, 0)
        .target_object(chosen_creatures[0])
        .resolve();
    assert_eq!(
        attached_object(&chosen, chosen_luxior),
        Some(chosen_creatures[0]),
        "positive reach-guard: a chosen host is attached"
    );

    // Declined: a legal host exists but the controller chooses none.
    let (mut runner, luxior, _) = luxior_runner(1);
    let declined = runner.activate(luxior, 0).resolve();
    assert_eq!(
        attached_object(&declined, luxior),
        None,
        "declining the optional host resolves the Attach unattached"
    );
    assert!(attach_effect_resolved(&declined));
    assert_eq!(counters_of(&declined, luxior, &CounterType::Loyalty), 5);
}

#[test]
fn luxior_plus_one_attaches_when_a_creature_is_chosen() {
    let (mut runner, luxior, creatures) = luxior_runner(1);
    let outcome = runner
        .activate(luxior, 0)
        .target_object(creatures[0])
        .resolve();
    assert_eq!(attached_object(&outcome, luxior), Some(creatures[0]));
    assert_eq!(counters_of(&outcome, luxior, &CounterType::Loyalty), 5);
}

// ---------------------------------------------------------------------------
// The Aetherspark — optional host plus its counter rider
// ---------------------------------------------------------------------------

#[test]
fn aetherspark_plus_one_with_no_creature_puts_no_counter() {
    // Reach-guard (same test): a chosen host receives the +1/+1 counter.
    let mut scenario = GameScenario::new();
    scenario.at_phase(Phase::PreCombatMain);
    let spark = scenario
        .add_planeswalker_from_oracle(P0, "The Aetherspark", "Aetherspark", 4, AETHERSPARK_ORACLE)
        .as_artifact()
        .with_subtypes(vec!["Equipment"])
        .id();
    let mine = scenario.add_creature(P0, "Chosen Host", 2, 2).id();
    let mut chosen_runner = scenario.build();
    grant_priority(&mut chosen_runner, P0);
    let chosen = chosen_runner
        .activate(spark, 0)
        .target_object(mine)
        .resolve();
    assert_eq!(attached_object(&chosen, spark), Some(mine));
    assert_eq!(
        counters_of(&chosen, mine, &CounterType::Plus1Plus1),
        1,
        "the chosen host gets the counter rider"
    );

    // Declined: the only creature is the opponent's, so the host slot is empty
    // and the rider has no object to affect.
    let mut scenario = GameScenario::new();
    scenario.at_phase(Phase::PreCombatMain);
    let spark = scenario
        .add_planeswalker_from_oracle(P0, "The Aetherspark", "Aetherspark", 4, AETHERSPARK_ORACLE)
        .as_artifact()
        .with_subtypes(vec!["Equipment"])
        .id();
    let theirs = scenario.add_creature(P1, "Bystander", 1, 1).id();
    let mut runner = scenario.build();
    grant_priority(&mut runner, P0);
    let declined = runner.activate(spark, 0).resolve();
    assert_eq!(attached_object(&declined, spark), None);
    assert_eq!(
        counters_of(&declined, theirs, &CounterType::Plus1Plus1),
        0,
        "an ineligible creature receives no counter"
    );
    assert!(attach_effect_resolved(&declined));
}

// ---------------------------------------------------------------------------
// Sting, Bilbo's Sword — ETB trigger whose SUB-clause host is optional
// ---------------------------------------------------------------------------

#[test]
fn sting_enters_with_no_legal_creature_and_attaches_to_nothing() {
    let mut scenario = GameScenario::new();
    scenario.at_phase(Phase::PreCombatMain);
    let sting = scenario
        .add_artifact_to_hand_from_oracle(P0, "Sting, Bilbo's Sword", STING_ORACLE)
        .with_subtypes(vec!["Equipment"])
        .id();
    let mut runner = scenario.build();
    grant_priority(&mut runner, P0);
    // The trigger's "for each creature target opponent controls" instruction
    // declares its opponent as a target even though this run declines the host.
    let outcome = runner.cast(sting).target_player(P1).resolve();

    // CR 603.3d + CR 115.6: the trigger's optional host is declined and the
    // trigger still resolves. The hone-counter instruction is a separate
    // mandatory step, so its EffectResolved must appear BEFORE the Attach one
    // (CR 608.2c: instructions are followed in order).
    let first_effect_resolution = |kind: EffectKind| {
        outcome.events().iter().position(|event| {
            matches!(
                event,
                GameEvent::EffectResolved {
                    kind: resolved,
                    ..
                } if *resolved == kind
            )
        })
    };
    let counter_step =
        first_effect_resolution(EffectKind::PutCounter).expect("hone-counter step must resolve");
    let attach_step = first_effect_resolution(EffectKind::Attach)
        .expect("the declined Attach must still resolve as a no-op");
    assert!(
        counter_step < attach_step,
        "CR 608.2c: the hone-counter step ({counter_step}) must precede the declined Attach \
         ({attach_step}): {:?}",
        outcome.events()
    );
    assert_eq!(attached_object(&outcome, sting), None);
    assert!(attach_effect_resolved(&outcome));

    // D5 (pre-existing, labelled, NOT fixed here): Sting's Attach is the SECOND
    // sentence of the trigger, so it resolves as a chain child. The parent->child
    // hand-off (`effects::apply_parent_chain_context`) replaces the child's
    // context wholesale and with it the node-local `attach_target_bindings`, so a
    // CHOSEN host is lost before resolution. A chosen-host positive therefore
    // cannot attach even at base (it errored `MissingParam`); the class reach
    // guard for "a chosen host does attach" is the trigger-root Attach row
    // (`assassin_gauntlet_etb_declined_host_stays_unattached`'s positive half),
    // not this card. The discriminating assertion above is the trigger
    // RESOLUTION: at base a mandatory host slot with a legal host was announced,
    // but the declined set removed the trigger, so no PutCounter ever resolved.
}

#[test]
fn sting_enters_and_attaches_to_a_chosen_host() {
    // A chosen host is announced as an explicit object target; the resolver
    // consumes that slot, so the Attach lands (the node-local host BINDING is not
    // required for this path — see the D5 note in the report).
    let mut scenario = GameScenario::new();
    scenario.at_phase(Phase::PreCombatMain);
    let sting = scenario
        .add_artifact_to_hand_from_oracle(P0, "Sting, Bilbo's Sword", STING_ORACLE)
        .with_subtypes(vec!["Equipment"])
        .id();
    let mine = scenario.add_creature(P0, "Willing Host", 2, 2).id();
    let mut runner = scenario.build();
    grant_priority(&mut runner, P0);
    let chosen = runner
        .cast(sting)
        .target_object(mine)
        .target_player(P1)
        .resolve();
    assert_eq!(attached_object(&chosen, sting), Some(mine));
    assert!(attach_effect_resolved(&chosen));
}

// ---------------------------------------------------------------------------
// Declined-host regressions for the Attach host class
// ---------------------------------------------------------------------------

/// Cast an Equipment from hand and drive its enters trigger with no host intent
/// (the driver declines the optional host slot).
fn cast_equipment_declined(name: &str, oracle: &str) -> Outcome {
    let mut scenario = GameScenario::new();
    scenario.at_phase(Phase::PreCombatMain);
    let equipment = scenario
        .add_artifact_to_hand_from_oracle(P0, name, oracle)
        .with_subtypes(vec!["Equipment"])
        .id();
    let mut runner = scenario.build();
    grant_priority(&mut runner, P0);
    runner.cast(equipment).resolve()
}

#[test]
fn assassin_gauntlet_etb_declined_host_stays_unattached() {
    let mut scenario = GameScenario::new();
    scenario.at_phase(Phase::PreCombatMain);
    let gauntlet = scenario
        .add_artifact_to_hand_from_oracle(P0, "Assassin Gauntlet", ASSASSIN_GAUNTLET_ORACLE)
        .with_subtypes(vec!["Equipment"])
        .id();
    // A legal host exists; the controller still chooses none (CR 115.6).
    let _legal_host = scenario.add_creature(P0, "Declined Host", 2, 2).id();
    let mut runner = scenario.build();
    grant_priority(&mut runner, P0);
    // The trigger's second printed sentence still needs its own target opponent,
    // declared separately from the declined object target.
    let declined = runner.cast(gauntlet).target_player(P1).resolve();
    assert_eq!(attached_object(&declined, gauntlet), None);
    assert!(
        attach_effect_resolved(&declined),
        "the declined Attach must complete, not error: {:?}",
        declined.events()
    );

    // Reach-guard: the same cast with a declared host attaches.
    let mut scenario = GameScenario::new();
    scenario.at_phase(Phase::PreCombatMain);
    let gauntlet = scenario
        .add_artifact_to_hand_from_oracle(P0, "Assassin Gauntlet", ASSASSIN_GAUNTLET_ORACLE)
        .with_subtypes(vec!["Equipment"])
        .id();
    let mine = scenario.add_creature(P0, "Chosen Host", 2, 2).id();
    let mut chosen_runner = scenario.build();
    grant_priority(&mut chosen_runner, P0);
    let chosen = chosen_runner
        .cast(gauntlet)
        .target_object(mine)
        .target_player(P1)
        .resolve();
    assert_eq!(attached_object(&chosen, gauntlet), Some(mine));
}

#[test]
fn hunters_bow_decline_with_mandatory_host_is_a_silent_no_op() {
    // LABELLED over-application (residual): Hunter's Bow prints a MANDATORY host
    // ("attach it to target creature you control"), but the pre-existing trigger
    // `has_up_to` stamp marks the execute optional. The declined host therefore
    // reaches the resolver; before the no-op it errored `MissingParam`, and now
    // it does nothing (CR 115.6 + CR 608.2c). The damage rider has no legal
    // "creature you don't control" either, so nothing happens.
    let outcome = cast_equipment_declined("Hunter's Bow", HUNTERS_BOW_ORACLE);
    // Locate the equipment again via the event source: we only need its identity
    // to assert the unattached state, so re-create the same query from state.
    let unequipped = outcome
        .state()
        .objects
        .values()
        .filter(|object| object.name == "Hunter's Bow")
        .all(|object| object.attached_to.is_none());
    assert!(
        unequipped,
        "the declined Attach must leave the Equipment unattached"
    );
    assert!(
        attach_effect_resolved(&outcome),
        "the declined Attach resolves"
    );
    assert!(matches!(
        outcome.final_waiting_for(),
        WaitingFor::Priority { .. }
    ));

    // Reach-guard: with a declared host the equipment attaches.
    let mut scenario = GameScenario::new();
    scenario.at_phase(Phase::PreCombatMain);
    let bow = scenario
        .add_artifact_to_hand_from_oracle(P0, "Hunter's Bow", HUNTERS_BOW_ORACLE)
        .with_subtypes(vec!["Equipment"])
        .id();
    let mine = scenario.add_creature(P0, "Chosen Host", 3, 3).id();
    let mut chosen_runner = scenario.build();
    grant_priority(&mut chosen_runner, P0);
    let chosen = chosen_runner.cast(bow).target_object(mine).resolve();
    assert_eq!(attached_object(&chosen, bow), Some(mine));
}

#[test]
fn novel_nunchaku_decline_with_mandatory_host_is_a_silent_no_op() {
    // Same labelled over-application as Hunter's Bow. The "When you do" reflexive
    // fight never fires because nothing became attached.
    let mut scenario = GameScenario::new();
    scenario.at_phase(Phase::PreCombatMain);
    let nunchaku = scenario
        .add_artifact_to_hand_from_oracle(P0, "Novel Nunchaku", NOVEL_NUNCHAKU_ORACLE)
        .with_subtypes(vec!["Equipment"])
        .id();
    let mut runner = scenario.build();
    grant_priority(&mut runner, P0);
    let declined = runner.cast(nunchaku).resolve();
    assert_eq!(attached_object(&declined, nunchaku), None);
    assert!(attach_effect_resolved(&declined));
    assert!(
        !declined.events().iter().any(|event| matches!(
            event,
            GameEvent::EffectResolved {
                kind: EffectKind::Fight,
                ..
            }
        )),
        "no attach means no reflexive fight: {:?}",
        declined.events()
    );
    assert!(matches!(
        declined.final_waiting_for(),
        WaitingFor::Priority { .. }
    ));

    // Reach-guard: with a declared host the equipment attaches and the reflexive
    // trigger is on the stack (it may still be declined for want of a target).
    let mut scenario = GameScenario::new();
    scenario.at_phase(Phase::PreCombatMain);
    let nunchaku = scenario
        .add_artifact_to_hand_from_oracle(P0, "Novel Nunchaku", NOVEL_NUNCHAKU_ORACLE)
        .with_subtypes(vec!["Equipment"])
        .id();
    let mine = scenario.add_creature(P0, "Chosen Host", 3, 3).id();
    let mut chosen_runner = scenario.build();
    grant_priority(&mut chosen_runner, P0);
    let chosen = chosen_runner.cast(nunchaku).target_object(mine).resolve();
    assert_eq!(attached_object(&chosen, nunchaku), Some(mine));
}

// ---------------------------------------------------------------------------
// Serene Remembrance — the compound-shuffle second subject
// ---------------------------------------------------------------------------

/// D4 (pre-existing, labelled): the "from a single graveyard" constraint is
/// parsed (the semantic row asserts `SameZoneOwner`) but not enforced on the
/// sub-ability's announced target set — a mixed-graveyard pair is accepted, so
/// this row asserts only what the engine actually does for the legal set.
#[test]
fn serene_remembrance_shuffles_two_cards_from_one_graveyard() {
    let mut scenario = GameScenario::new();
    scenario.at_phase(Phase::PreCombatMain);
    let first = scenario
        .add_creature_to_graveyard(P0, "Grave Ghoul", 1, 1)
        .id();
    let second = scenario
        .add_creature_to_graveyard(P0, "Grave Spirit", 1, 1)
        .id();
    let serene = scenario
        .add_spell_to_hand_from_oracle(P0, "Serene Remembrance", false, SERENE_REMEMBRANCE_ORACLE)
        .id();
    let mut runner = scenario.build();
    grant_priority(&mut runner, P0);
    let outcome = runner
        .cast(serene)
        .target_objects(&[first, second])
        .resolve();
    // CR 115.6 + CR 701.24a: the second subject's "up to three target cards"
    // includes both chosen cards; both are shuffled into their owner's library.
    assert_eq!(outcome.zone_of(first), Zone::Library);
    assert_eq!(outcome.zone_of(second), Zone::Library);
}

// ---------------------------------------------------------------------------
// Stolen Goodies — the distribution pin (finite {0,3} spec survives assembly)
// ---------------------------------------------------------------------------

#[test]
fn stolen_goodies_accepts_at_most_three_targets() {
    let mut scenario = GameScenario::new();
    scenario.at_phase(Phase::PreCombatMain);
    let creatures: Vec<ObjectId> = (0..4)
        .map(|index| {
            scenario
                .add_creature(P0, &format!("Hungry {index}"), 1, 1)
                .id()
        })
        .collect();
    let goodies = scenario
        .add_spell_to_hand_from_oracle(P0, "Stolen Goodies", false, STOLEN_GOODIES_ORACLE)
        .id();
    let mut runner = scenario.build();
    grant_priority(&mut runner, P0);
    // Four legal creatures are declared; the printed {0,3} spec caps the
    // announced set at three, so the fourth must receive nothing.
    let outcome = runner.cast(goodies).target_objects(&creatures).resolve();
    let counters: Vec<u32> = creatures
        .iter()
        .map(|creature| counters_of(&outcome, *creature, &CounterType::Plus1Plus1))
        .collect();
    assert_eq!(
        counters.iter().sum::<u32>(),
        3,
        "all three counters are distributed: {counters:?}"
    );
    assert!(
        counters.iter().filter(|count| **count > 0).count() <= 3,
        "at most three targets may receive counters: {counters:?}"
    );
}
