# 600 More: Colonists Who Actually Play, and the Romance That Falls Out

**Status:** v1.0 · **Date:** 2026-09-06 · **Items 1401–2000**
**Continues:** `LANDSCAPE_200.md`, `LANDSCAPE_800.md`, `LANDSCAPE_1400.md`
**Revises:** `PD_BRIEF.md` §3 (P3), §6 (Cast), §7 (Multiplayer)

---

## The idea that changes P3

`PD_BRIEF.md` calls the relationship pillar "deep, branching, stat- and
memory-driven romance." That was written before we had a biosphere. It assumes
the Stardew model: NPCs on schedules, affinity from gifts, dialogue unlocked at
thresholds.

**Throw it out.** We now have a habitat where drainage reroutes when you dig,
soil remembers what you grew, and a chronicle that records what happened where.
So:

> **Make the colonists agents that play the same game you do, through the same
> API you do. Then romance stops being a dialogue tree and becomes a working
> relationship.**

You do not learn who someone is by exhausting their conversation topics. You
learn it by watching what he does with land. Whether he terraces or strip-mines.
Whether he plants for himself or upstream of you. Whether he came when your
levee failed.

And the bond depth that gates the Crèche (`PD_BRIEF.md` §4) is then **earned
through shared material history**, not accumulated through gifts. That is a
better romance mechanic than any dating sim has, and it is only available to us
because of what the last three documents built.

### The chronicle is the relationship database

Item 1141 gave us a per-region event chronicle. It turns out to be the memory
ledger `PD_BRIEF.md` §6 asked for. "You dug the channel that saved my field on
day 212" is a *query*, not authored dialogue. Every romance beat has a citation.

### The architecture fork, and my recommendation

There is one decision here that everything else depends on.

**Recommendation: agents plan with utility AI / GOAP. Language is a separate,
optional layer that voices decisions it did not make.**

| | Sim-driven agents (recommended) | LLM-driven agents |
|---|---|---|
| Determinism | Yes — replay, saves, golden tests survive (`REQUIREMENTS.md` B8) | No |
| Offline | Yes — `EM_BRIEF.md` §2.3 requires local-first | Needs a local model or a network |
| Cost per hour | Zero | Real, per player, forever |
| 12 agents at 60 fps | Trivial | Not without heavy batching |
| Can hallucinate world facts | No | **Yes, and fatally** |

That last row decides it. An LLM agent will tell you it dug a channel it did not
dig. In a game whose entire proposition is honesty of mechanism, an NPC that
misremembers the landscape is worse than no NPC. **The simulation decides what
they do; the language layer only says why.** Grounded in real state, with the
chronicle as its only source of fact.

### What this does to multiplayer

It resolves `PD_BRIEF.md` §7's awkwardness. The rule was "never player↔player
romance" for moderation and age-verification reasons — correct, but it left the
social pillar thin. With agents:

- **You romance agents.** Safe in multiplayer, no consent surface between real
  people, and it never touches the Archive's content policy.
- **You collaborate with players.** Neighbours, trade partners, upstream and
  downstream of each other.
- **Agents keep your habitat alive while you are offline**, which is what makes
  async Shared Habitat feel like a place rather than a save file.

---

## AH. Agent colonists — they play the same game (1401–1490)

*The rule that makes all of this honest: an agent may only do what the player
can do, through the same functions.*

1401. **Agents act through the player's API.** `dig`, `fill`, `place_module`,
      harvest, carry. No agent-only affordances, ever.
1402. Which means every agent action is already persisted as strokes, soil
      deltas and modules — no separate save path.
1403. And every agent action is already visible in the world, because it is the
      same world.
1404. An agent body identical to the player's rig, with the same procedural gait
      and the same collision against the density field.
1405. Agents subject to spin gravity, encumbrance and terrain exactly as you are.
1406. Agents that get stuck, and notice, and route around — because the terrain
      is not authored for them.
1407. Agents with an inventory obeying the mass and volume limits of §X item 820.
1408. Agents that must carry material to where it is needed, so transport
      infrastructure benefits them too.
1409. Agent labour as the labour constraint in §Y item 943 — the colony's
      capacity is the sum of its agents.
1410. A daily rhythm from the light schedule, not from a hardcoded clock.
1411. Sleep, meals and rest as real needs with real time cost.
1412. Fatigue reducing work rate and raising mistake probability.
1413. Hunger drawing from actual stores, which can actually be empty.
1414. Illness and injury as states with recovery times.
1415. Injury from the world: rockfall, collapse, a fall, a flood.
1416. **Death as permanent**, with the colony genuinely poorer for it.
1417. Which makes rescuing someone a real act with real stakes.
1418. Agents whose work you can find evidence of: fresh spoil, a new ditch, a
      half-built wall.
1419. Work-in-progress state, so you can see what someone is partway through.
1420. Agents finishing what another started, if it makes sense to.
1421. Agents abandoning a plan when conditions change, and saying so.
1422. Territory: each agent has ground they consider theirs, emerging from where
      they have worked.
1423. Which produces **upstream and downstream neighbours with material
      interests** (§I item 170) who are people, not factions.
1424. Agents petitioning each other over water, timber and soil.
1425. Agents who cooperate on infrastructure too large for one person.
1426. Barn-raising: a build that needs several agents and can include you.
1427. Agents who reciprocate help, tracked as an actual ledger of favours.
1428. Agents who remember being helped, and being ignored.
1429. Specialisation emerging from what each agent has practised (§AF item 1314).
1430. A smith who is a smith because he smelted for two hundred days.
1431. Skill visible in output quality, so a good farmer's field looks better.
1432. Apprenticeship between agents, transferring skill.
1433. Knowledge loss when a skilled agent dies (§AF item 1315).
1434. Agents teaching the player, and the player teaching them.
1435. Agents who are bad at things, and know it, and ask.
1436. Agents with material preferences — one likes working stone, one hates it.
1437. Agents with land preferences: the high ground, the riverside, the woods.
1438. Which means where someone chooses to live tells you about them.
1439. Agents who improve their own plot over years, visibly.
1440. Agents whose plot degrades if they are careless, visibly.
1441. Careless agents as a real problem you can see coming and try to address.
1442. Agents making genuinely bad decisions, occasionally, for reasons.
1443. Agents disagreeing with you about land management, with a real argument.
1444. Agents who are right when you are wrong, and the game not protecting you.
1445. Agents who adopt a technique after seeing it work on your land.
1446. Diffusion of practice through the colony as an observable process.
1447. Agents who resist a technique because it failed for them once.
1448. Reputation among agents from material acts (§AF item 1335).
1449. Gossip: agents telling each other what they saw you do.
1450. Which means reputation propagates through a social graph with delay and
      distortion, rather than being a global number.
1451. Agents who trust your judgement and follow your lead.
1452. Agents who do not, and go their own way.
1453. Agents who leave a settlement and found another.
1454. Settlement formation as an emergent agent decision (§AF item 1322).
1455. Agents who visit your fields to look, and comment on what they see.
1456. Comments grounded in the actual field state, always.
1457. Agents noticing change: "your soil is darker than last season."
1458. Agents noticing the landscape: the new channel, the felled wood, the fire
      scar.
1459. Agents reacting to weather with the same information you have.
1460. Agents preparing for a forecastable storm, because the weather is a
      machine and they can read it too (§D item 80).
1461. Agents failing to prepare, and suffering, and remembering.
1462. Agents at the Committee arguing from their own material interest.
1463. Agents voting on daylight length and power allocation (§AF item 1331).
1464. Agents whose vote you can predict once you know their land.
1465. Which makes politics legible without a UI explaining it.
1466. Agents who campaign, persuade and trade votes.
1467. Agents holding grudges over water rights.
1468. Agents forming alliances from shared watershed position.
1469. Agents whose relationships with *each other* evolve independently of you
      (`PD_BRIEF.md` §6).
1470. Which means walking into a room and finding a situation you did not cause.
1471. Agents pairing off with each other if you do not pursue them.
1472. Which makes the romance genuinely competitive without a mechanic saying so.
1473. Agents grieving a lost partner, and eventually not.
1474. Agents grieving a lost *place* (§AF item 1340) — the flooded field, the
      burnt wood.
1475. Agents with attachment to specific trees, views and buildings.
1476. Agents who plant something for someone.
1477. Agents who build a bench where the view is good (§AD item 1269).
1478. Agents naming places, and the names sticking (§AC item 1166).
1479. Agents writing in the chronicle, so the record is not only yours.
1480. Agents whose journals you can find (§AC item 1179).
1481. Agents who record in the Archive, so it holds more than your memories.
1482. Twelve agents at T0 near the player; the rest as statistical population.
1483. Agent promotion and demotion using the same tier mechanism as everything
      else (`SIM_ARCH_BRIEF.md` §1).
1484. Off-screen agents advancing statistically, so returning shows real change.
1485. Which is how the world stays alive without simulating a hundred people at
      full fidelity.
1486. Agent count as a performance dial, honestly documented.
1487. Agents running headless in the same runner as everything else (§W item
      787), so a hundred simulated days of colony life is a test you can run.
1488. Colony outcomes across seeds as a distribution (§W item 788) — does a
      colony of twelve agents survive without a player at all?
1489. If it does not, that is a balance finding, not a bug.
1490. Publish the limits: agents as utility planners, twelve at fidelity, no
      theory of mind beyond a relationship ledger.

## AI. Perception, planning, skill and personality (1491–1560)

*What an agent knows, how it decides, and why two agents are different.*

1491. **Agents perceive through a modelled sensory environment**, following
      ORRERY's `sensoryEnvAt` — not by reading global state.
1492. Line of sight against the terrain, so an agent can be surprised.
1493. Hearing with distance falloff and terrain occlusion (§AD item 1219).
1494. A knowledge model per agent: what they have surveyed, and when.
1495. Stale knowledge, so an agent may act on a field state that has changed.
1496. Which produces honest mistakes rather than omniscient competence.
1497. Agents surveying deliberately when their knowledge is old.
1498. Knowledge sharing through conversation, propagating discoveries.
1499. Rumour: knowledge that arrives distorted.
1500. Agents who have never been to a region having no opinion about it.
1501. A utility model over needs: food, water, warmth, safety, belonging,
      purpose.
1502. Action scoring from expected utility against the *agent's* knowledge.
1503. GOAP-style planning for multi-step goals: to make a pipe, get clay, get
      fuel, build a kiln.
1504. Plans that read the same recipe data the player's crafting does (§X item
      831).
1505. Plan invalidation when the world changes, with a visible switch.
1506. Plan horizon by personality: some agents plan a season, some plan a day.
1507. Long-horizon projects an agent returns to over many days.
1508. Agents who see a project through, and agents who do not.
1509. Opportunism: an agent passing a ripe crop picks it.
1510. Habits — routes and routines that persist because they worked.
1511. Habit disruption when the landscape changes under them.
1512. Risk tolerance as a trait, affecting whether an agent works a steep slope.
1513. Time preference: whether an agent plants an orchard or an annual.
1514. Which is the single most revealing trait a farming agent can have.
1515. Conscientiousness affecting maintenance and therefore their land's state.
1516. Sociability affecting how much they seek company and cooperation.
1517. Curiosity affecting how much they survey and experiment.
1518. Stubbornness affecting how long they persist with a failing plan.
1519. Traits expressed through *behaviour the player can observe*, never through
      a stat sheet.
1520. Which means you infer personality the way you do in life — from a pattern
      of choices.
1521. Traits drifting slowly with experience, so people change.
1522. Trauma from specific events, changing behaviour durably.
1523. An agent who was in a collapse avoiding tunnels afterward.
1524. An agent who lost a harvest to flood overbuilding levees forever.
1525. Recovery from trauma over long time, incompletely.
1526. Mood as a fast-moving state over slow traits.
1527. Mood affected by weather, food, sleep, company and success.
1528. Mood affecting work rate, sociability and generosity.
1529. Mood visible in posture, gait and idle animation, using the procedural rig.
1530. Which means you can read someone's day from across a field.
1531. Emotional expression through action: someone who is upset works alone.
1532. Comfort behaviours — a favourite place, a repeated task.
1533. Agents seeking each other out when low.
1534. Agents seeking *you* out, which should feel like being chosen.
1535. Bids for attention that you can miss.
1536. Which means the romance can be lost by inattention rather than by a wrong
      dialogue pick.
1537. Agents who notice being ignored, and withdraw.
1538. Repair after a rupture, requiring effort rather than a gift.
1539. Personality-driven work-style differences visible in their land.
1540. A terracer and a strip-miner producing visibly different hillsides.
1541. Which makes the landscape a record of everyone's character (§AC item 1182).
1542. Agent goals that conflict with the colony's, sometimes.
1543. Agents who prioritise their own plot over a shared levee.
1544. Free-rider dynamics on common resources (§AF item 1329).
1545. Agents who police the commons socially.
1546. Norms emerging from repeated interaction, and enforcement of them.
1547. Agents who break norms and are sanctioned.
1548. Sanctions the player can join, resist or arbitrate (§AF item 1334).
1549. Agents whose ethics differ on specific questions — the forest, the Crèche.
1550. Which gives the colony genuine moral disagreement without authored sides.
1551. Deliberation modelled as opinion exchange with weights.
1552. Opinion change as an observable process over days.
1553. Agents citing evidence from the chronicle in argument.
1554. Which means being right is persuasive, eventually.
1555. Agents who are persuaded by demonstration rather than argument.
1556. So the most effective politics is to farm well and let them look.
1557. An inspector showing an agent's current plan, needs and knowledge — a
      developer tool that doubles as an accessibility feature.
1558. Agent decision logs for debugging, and for the chronicle.
1559. Determinism across agent decisions, tested (§AQ).
1560. Publish the limits: utility planning with GOAP, traits as scalars, no
      learning beyond habit reinforcement.

## AJ. Relationships built from shared work (1561–1650)

*The heart of it. No affinity bar, no gift table. The chronicle is the ledger.*

1561. **Affinity derived from shared material history**, queried from the
      chronicle, not accumulated as a number.
1562. Which means every relationship state has a citation you can read.
1563. "You dug the channel that saved my field on day 212" as a generated line
      with a real event behind it.
1564. Weighting by cost to you: help that was expensive counts more.
1565. Weighting by need at the time: help during a crisis counts far more.
1566. Weighting by whether you were asked or came unprompted.
1567. Unprompted help as the strongest signal in the system, because it is.
1568. Help that was easy for you counting less, honestly.
1569. Gifts counting for very little, deliberately — this is not that game.
1570. Except gifts that are *materially significant*: the strain that survived
      the drought, the last of your seed.
1571. Which makes generosity legible by what it cost, not by what it was.
1572. Shared labour as the primary bonding mechanic: working the same field.
1573. Time worked together tracked per pair, per place.
1574. Places that are "yours together", emerging from that record.
1575. Returning to such a place having weight.
1576. Projects completed together, remembered as projects.
1577. Failure survived together counting more than success.
1578. Betrayal as a material act: taking their water, felling their wood.
1579. Which means you can hurt someone without a dialogue option.
1580. Accidental harm — your channel flooded his field — treated as harm, then
      contextualised by whether you fixed it.
1581. Restitution as a real action with real cost.
1582. Forgiveness over time, incompletely.
1583. Grudges with a decay curve and a floor.
1584. Trust as a separate axis from affection, moving differently.
1585. Respect as a third, earned by competence rather than kindness.
1586. Which lets an agent respect you and not like you, and vice versa.
1587. Attraction as a fourth axis, with its own drivers.
1588. All four visible to the player only through behaviour and dialogue, never
      as bars.
1589. Though an optional accessibility readout, off by default.
1590. Relationship *stage* as a derived state from the four axes, not a level.
1591. Stage transitions as events the chronicle records.
1592. Which means the romance has a history you can re-read.
1593. Agents with a Crèche stance (`PD_BRIEF.md` §6) that is a real position:
      whether they want a lineage, and with whom.
1594. Stance changing with circumstance — a bad year makes people cautious.
1595. Agents who want a lineage with someone else, and say so.
1596. Which makes rejection possible, specific and survivable.
1597. Rejection with a reason grounded in state, always.
1598. Agents you cannot romance, because they do not want you, and that being
      fine.
1599. Agents whose interest you did not notice until it was gone.
1600. Jealousy between agents as an emergent consequence of their own
      relationships (`PD_BRIEF.md` §7's Committee, without needing players).
1601. Agents discussing you with each other, changing each other's view.
1602. Which means the social graph mediates the romance.
1603. Public and private behaviour differing, and being noticed.
1604. Agents who are different when alone with you.
1605. Intimacy as increased disclosure: they tell you what they are actually
      worried about.
1606. Disclosure grounded in their real state — their plot, their fear, their
      plan.
1607. Which means intimacy is *informational*, and it is real information.
1608. Vulnerability as asking for help they cannot repay.
1609. Being asked as the clearest sign of trust in the system.
1610. Agents who will not ask, and the work of getting them to.
1611. Conflict within a relationship over land management.
1612. Which is a real argument about a real thing, not a manufactured beat.
1613. Compromise that changes what you both do afterwards.
1614. Shared plots — land you work jointly, with joint decisions.
1615. Which is a mechanical commitment as well as a romantic one.
1616. Living together as a change in daily routine you can observe.
1617. Household as a unit in the labour and consumption model.
1618. Shared inventory and shared stores, with the trust that implies.
1619. Domestic detail: who cooks, who repairs, what the house looks like.
1620. The house changing to reflect both of them.
1621. Objects in the house with histories — the tool you gave him.
1622. Agents keeping things you gave them, visibly.
1623. Agents putting your gift somewhere prominent, or not.
1624. Anniversaries of chronicle events, remembered unprompted.
1625. "A year since the flood" as a line delivered because it is true.
1626. Long-term partnership as a state with its own texture — comfort, routine,
      occasional friction.
1627. Partnership that can decay through neglect over seasons.
1628. Separation as a possible, survivable outcome.
1629. Which requires the game not to punish it mechanically.
1630. Reconciliation as a long project.
1631. Multiple simultaneous relationships as a thing the agents have opinions
      about, differing by agent.
1632. Which lets the colony's norms about this emerge rather than be legislated.
1633. Agents whose boundaries you can violate, with consequence.
1634. Explicit consent as a modelled state, always affirmative, never assumed.
1635. Which is both correct and mechanically clearer than a threshold.
1636. Refusal at any stage, respected by the system without penalty to the agent.
1637. Player-side consent controls for content, granular and honoured.
1638. Relationship inspector showing history rather than statistics.
1639. A relationship's chronicle as a readable timeline.
1640. Which is the same system as §AC, and that unification is the point.
1641. Romance requiring *both* the bond and the material base, per
      `PD_BRIEF.md` §4's Crèche gate.
1642. So neither pure farming nor pure courtship reaches a good ending.
1643. Agents who notice when you are neglecting your land for them.
1644. And who mind, because the colony needs the harvest.
1645. Which makes the two pillars pull against each other, deliberately.
1646. Time as the real currency: a day spent with him is a day not spent
      draining the field.
1647. Which is the entire game in one sentence.
1648. Relationship outcomes visible in the ending state (§AF item 1351).
1649. Descendants carrying both genomes and both reputations (§AF item 1349).
1650. Publish the limits: four scalar axes over a queried event history, no
      simulated emotion, no theory of mind.

## AK. Talk — the language layer, honestly bounded (1651–1710)

*Optional, grounded, and never in charge. If this layer vanishes, the game still
works.*

1651. **The language layer never decides actions.** It reports decisions the
      planner already made.
1652. And never asserts world facts. Every factual claim comes from a query.
1653. A structured intent layer between agent and dialogue: the agent emits
      `(intent, subject, evidence)` and language renders it.
1654. Which means the same intent can be rendered by a template or by a model,
      and the game does not care.
1655. **Templates as the shipping default**, so the game is fully playable
      offline with no model at all (`EM_BRIEF.md` §2.1).
1656. Templates written by a writer in Ink, keyed by intent and personality.
1657. Which is `EM_BRIEF.md`'s narrative pipeline, reused rather than replaced.
1658. Evidence slots filled from the chronicle, so lines cite real events.
1659. Numbers in dialogue pulled live: "your soil nitrogen is down a third."
1660. Which makes an agent a diegetic instrument as well as a person.
1661. Personality-conditioned phrasing from the trait vector.
1662. Mood-conditioned phrasing from the mood state.
1663. Relationship-stage-conditioned register, from formal to intimate.
1664. Regional dialect by band, since the drum's bands are isolated (§E item 90).
1665. Which is a linguistics detail that falls out of the climate model.
1666. An optional model layer that *paraphrases* template output for variety.
1667. Constrained to the same facts, verified by comparing extracted claims
      against the query results.
1668. A claim verifier that rejects any generated line asserting something false.
1669. Which is the only responsible way to use generation in a simulation game.
1670. Fallback to the template when verification fails, silently.
1671. A local small model as the preferred deployment, for offline and cost.
1672. Cloud as an optional enhancement with graceful degradation.
1673. Latency budgeted so conversation never blocks play.
1674. Streaming so a reply begins immediately.
1675. Cost per session measured and displayed to the developer, not hidden.
1676. Determinism preserved by keeping generation out of the simulation loop
      entirely.
1677. Which means a replay reproduces actions exactly and dialogue approximately,
      and that is the correct trade.
1678. Conversation as an activity with time cost, not a free menu.
1679. Which means talking to someone is a choice against draining a field.
1680. Topic availability from shared history rather than from unlock thresholds.
1681. So you can only discuss the flood if there was a flood.
1682. Player dialogue as intents rather than authored lines, in the same
      structure.
1683. Which keeps the player's voice consistent and avoids writing thousands of
      lines.
1684. Free-text input as an option, parsed to intent, never to arbitrary effect.
1685. Refusal to parse handled gracefully and honestly.
1686. Ambient conversation between agents, overheard, about real events.
1687. Which is how gossip (§AH item 1449) becomes audible.
1688. Conversations you interrupt, with the agents reacting to being interrupted.
1689. Conversations that continue without you, and that you can ask about.
1690. Agents asking *you* questions, and remembering the answers.
1691. Answers that matter materially: "will you take the upstream plot?"
1692. Which makes dialogue a commitment device.
1693. Promises tracked, and kept or broken observably.
1694. Broken promises weighted heavily in trust (§AJ item 1584).
1695. Agents holding you to your word, citing when you gave it.
1696. Non-verbal communication doing much of the work: proximity, gaze,
      posture, whether he keeps working while you talk.
1697. Silence as a legitimate and readable response.
1698. Voice acting for the authored cast only, with the procedural layer
      subtitled.
1699. Which bounds the recording budget to the cast in §AL.
1700. Barks and short vocalisations for everyone, generated and pitched per agent.
1701. Text presentation that respects reading speed and accessibility settings.
1702. Full text log of every conversation, searchable.
1703. Which is also the chronicle's social half.
1704. Translation and localisation planned from the intent layer, not from
      strings.
1705. Which is a genuine advantage of the structured approach.
1706. Content filters on any generated text, with a documented policy.
1707. No generated content in the intimacy layer, ever — that is §AL's authored
      territory.
1708. A writer's tool for authoring and testing intents against real world
      states.
1709. Dialogue tests in CI: every intent must render for every personality.
1710. Publish the limits: intent-driven templates, optional paraphrase, verified
      claims, no open-ended reasoning.

## AL. The authored cast and the Archive (1711–1770)

*Twelve people with names, faces and footage — who are agents like everyone else.*

1711. **The named cast are agents too.** Same planner, same API, same needs.
      Authored personality, portrait, voice and Archive footage on top.
1712. Which resolves the tension between procedural life and
      `PD_BRIEF.md` §8.2's six real actors.
1713. Unnamed colonists as agents without romance arcs or footage.
1714. Six romanceable at Option A scope, twelve at full (`PD_BRIEF.md` §10).
1715. Each with an authored trait vector, so their behaviour is *characteristic*
      rather than random.
1716. Each with an authored starting plot, chosen to express them.
1717. The hydroponics chief on the wet flat; the reactor engineer near the hull.
1718. Which means you meet them through their land before you meet them.
1719. Authored arc beats gated on **material conditions**, not on affinity points.
1720. So his arc advances when the thing he feared actually happens.
1721. Which requires beats written against world states, and a validator that
      checks each is reachable.
1722. Arcs that can be missed entirely if the condition never arises.
1723. And that is acceptable, because the world is honest.
1724. Fallback beats for arcs the world has not triggered by late game.
1725. **Archetype as entry read, subverted by arc** (`PD_BRIEF.md` §6) — and now
      subverted through behaviour, which is stronger than through dialogue.
1726. The bear-ish hydroponics chief who is meticulous and anxious.
1727. The leather-clad reactor engineer who is the most cautious person in the
      colony.
1728. Which the player discovers by watching him work, not by being told.
1729. The sociologist NPC who says the archetype thing out loud, as designed.
1730. Cast members with opinions about each other, evolving independently.
1731. Cast members who pair off with each other if you do not pursue them
      (§AH item 1471).
1732. Which makes the cast feel like a community rather than a menu.
1733. Cast deaths as permanent and devastating, with their arc unfinished.
1734. Which the game should not soften.
1735. **The Archive as the colony's memory system** (`PD_BRIEF.md` §8.1), fed by
      the chronicle (§AC item 1175).
1736. Which means the Archive holds far more than intimacy — it holds the flood,
      the fire, the first harvest.
1737. Live-action footage reserved for the intimacy peaks of authored arcs.
1738. Framed diegetically as ship records, per §8.1.
1739. Aspect change, grain, timecode and a fixed slightly-wrong camera.
1740. Archive browsing as a real interface, including earlier lineages.
1741. Including footage from generations before the player.
1742. Which gives the world a past it did not need the player for.
1743. Archive entries with metadata: who, where, when, and what was happening in
      the colony that day.
1744. Which contextualises every scene against the material state.
1745. So an intimacy scene during a drought reads differently, automatically.
1746. Archive search by person, place, date and event.
1747. Archive degradation with age, as a diegetic and aesthetic choice.
1748. Player-recorded Archive entries of their own choosing.
1749. Which lets the player author into the fiction.
1750. Consent gates on Archive recording, in fiction and in interface.
1751. Content settings that disable the footage layer entirely, with the arcs
      still completing in-engine.
1752. Which is required, not optional — some players will not want it and the
      story must still work.
1753. In-engine stylised intimacy as the default path (`PD_BRIEF.md` §8), well
      directed.
1754. The footage layer as a separate download, so the base build is unaffected.
1755. Casting the actor who is also the photoscan and the voice
      (`PD_BRIEF.md` §8.2), which stays the plan.
1756. Casting therefore upstream of character art (`EM_BRIEF.md` R4).
1757. Stylised cast per `EM_BRIEF.md` §2.4, which makes the medium shift
      intentional rather than a fidelity claim.
1758. S4 still the gate on all of it.
1759. Cast members whose romance requires the Crèche's material base
      (`PD_BRIEF.md` §4), so the pillars remain coupled.
1760. Cast members who will not commit until the colony is secure.
1761. Which turns "fix the drainage" into a romance action.
1762. And that is the sentence the whole design has been reaching for.
1763. Cast arcs that comment on the player's land management, in character.
1764. Cast arcs that diverge based on how the colony is faring.
1765. A good colony and a failing one producing different versions of the same
      relationship.
1766. Endings per cast member, tied to the colony's state.
1767. Which means there is no romance ending independent of the habitat.
1768. Cast members in the ending report, and their descendants.
1769. A cast bible as a living document alongside the arc validator.
1770. Publish the limits: twelve authored arcs, beats gated on world state,
      footage for intimacy peaks only.

## AM. The Crèche, lineage and consequence (1771–1820)

*`PD_BRIEF.md` §4 said the Crèche is what makes this one game. With agents and a
working biosphere, it finally can be.*

1771. **Crèche slots gated by real carrying capacity** — arable area times actual
      yield, from the soil grid (§AF item 1320).
1772. Which means a Crèche slot is literally a field you drained.
1773. Slot cost in nutrients, calories and labour, drawn from the ledger (§J).
1774. Genome pairing quality from the two agents' actual genomes (§AB item 1081).
1775. Inbreeding coefficient computed from the real pedigree.
1776. Which becomes the colony's central genetic problem over generations
      (§AB item 1098).
1777. Genetic load rising visibly across the voyage, on a graph.
1778. Radiation exposure per lineage tracked, tying to the founding premise.
1779. Shielding as an infrastructure investment against it.
1780. The Crèche committee as agents with positions (§AH item 1462).
1781. Positions derived from their material interest and their ethics.
1782. Which makes the committee's decision predictable but not controllable.
1783. Your standing with the committee earned through the colony's condition.
1784. So the way to a lineage is to make the colony able to afford one.
1785. Competing applications from other pairs, agent and player alike.
1786. Rejection with a stated, honest reason.
1787. Reapplication after changing the conditions that caused rejection.
1788. Which converts a romance goal into a terraforming project.
1789. Lineage as an actual entity with a genome, a name and a chronicle.
1790. Children as agents who grow, learn and specialise (§AH item 1429).
1791. Inheriting traits from both parents, expressed not averaged.
1792. Inheriting *land* — the plot you improved becomes theirs.
1793. Which is why item 1182 matters: they inherit your mistakes too.
1794. Children who ask about the chronicle entries you are in.
1795. Children who make different choices than you did.
1796. Generational play as an option (`PD_BRIEF.md` §11's open question), with
      your descendant as the next protagonist.
1797. The previous player character as an agent, or as a chronicle entry.
1798. Their grave, their bench, their orchard.
1799. Multi-generational soil and drainage trends spanning the whole playthrough.
1800. Population pyramid as a readout, with dependency ratio (§AF item 1319).
1801. Demographic collapse as a slow, visible, avoidable failure.
1802. Demographic overshoot as its opposite, equally visible.
1803. Which means the Crèche is a resource decision with a fifty-day feedback
      delay and a fifty-year consequence.
1804. The colony debating its own growth rate, with real numbers.
1805. Factions on that question forming from material position.
1806. Which is the most thematically loaded politics available to this setting.
1807. Adoption and shared parenting as recognised structures.
1808. Non-reproductive partnership as equally valid and equally supported.
1809. Which the fiction should be explicit about, given the premise.
1810. The Crèche's technology as visible infrastructure with a maintenance cost.
1811. Crèche failure as a genuine crisis.
1812. Gamete banking as insurance (§AB item 1105).
1813. Loss of the bank as a catastrophe with a century-long shadow.
1814. Founder manifest versus current population (§AB item 1135).
1815. Lineages that ended, recorded and remembered.
1816. Lineages that thrived, and why.
1817. A lineage viewer showing the whole colony's pedigree as a graph.
1818. Which is the phylogeny tool (§AB item 1126) applied to people.
1819. Arrival state defined by who is aboard and what they inherited
      (§AF item 1352).
1820. Publish the limits: pedigree with an inbreeding coefficient, traits as an
      expressed vector, carrying capacity from a yield estimate.

## AN. Multiplayer — the shared habitat (1821–1900)

*`PD_BRIEF.md` §7's async model, made worth playing by the fact that agents keep
your drum alive while you are gone.*

1821. **Async Shared Habitat as v1**, unchanged from `PD_BRIEF.md` §7. Visit,
      trade, gift, collaborate.
1822. Each player owning one habitat, authoritative for its own state.
1823. Agents continuing to work it while the owner is offline (§AH item 1484).
1824. Which means returning shows real change caused by people, not a timer.
1825. Habitat state summarised for visitors: biome census, budgets, chronicle.
1826. Which is the survey view (`LANDSCAPE_800.md`) reused as a social object.
1827. Visiting as a real trip with real presence, seeing their land and their
      agents.
1828. Your reputation with *their* agents, built by what you do on their land.
1829. Which means visiting badly has consequences.
1830. Guest permissions granular by default: look, walk, work, dig, build.
1831. Nothing destructive without explicit grant, ever.
1832. An undo log for guest actions the host can review and revert.
1833. Which is the only responsible design for a mutable shared world.
1834. Gifting material, with the mass leaving your closed ledger and entering
      theirs.
1835. Which is the only way mass crosses between drums, and should feel like it.
1836. Trade with agreed terms, escrowed.
1837. Seed and strain exchange, carrying real genetics (§AB item 1103).
1838. Which makes another player's drought-tolerant landrace genuinely valuable.
1839. Strains recording their provenance — who bred it, where, when.
1840. A strain's pedigree crossing between drums, building a shared history.
1841. Which is a beautiful, low-risk, high-retention social system.
1842. Technique sharing: a blueprint of a terrace, a channel, a kiln.
1843. Blueprints as data the host can place, adapted to their terrain.
1844. Which requires the placement to re-solve against local ground.
1845. Co-op builds on a host's land, with contribution tracked.
1846. Barn-raising across players (§AH item 1426).
1847. Shared projects too large for one colony.
1848. Agent visitors travelling between drums, carrying news and gossip.
1849. Which propagates reputation between players through agents.
1850. Agents who report what they saw on your neighbour's land.
1851. Which is a social feed that is diegetic rather than a UI.
1852. Asynchronous messaging left as physical notes in places.
1853. Notes attached to features: "this is where the levee failed."
1854. Which is Dark Souls' message system, grounded in a real landscape.
1855. Note voting and moderation, lightweight.
1856. A neighbourhood of a handful of drums, so relationships are durable.
1857. Which beats a global pool for retention and for safety.
1858. Neighbourhood-scale challenges and comparisons.
1859. A shared leaderboard on things that are actually interesting: arable
      hectares, soil carbon, species retained.
1860. Which rewards stewardship rather than extraction.
1861. Anti-metrics displayed too: soil lost, species extinct.
1862. Which the design should be brave enough to show.
1863. Photo and Archive sharing between players (§T item 677).
1864. A visitor's own chronicle entry on your land, so visits are remembered.
1865. Long absence handled honestly: agents cope, or do not, and you see which.
1866. Which is the one honest answer to the idle-decay problem.
1867. Catch-up summaries on return: what happened, what failed, who did what.
1868. Which is the chronicle, filtered, and it is the best possible return
      experience.
1869. Optional pause-on-absence for players who want it, clearly labelled.
1870. No pay-to-skip on anything, ever, given the design's premises.
1871. Deterministic simulation making server-side verification tractable.
1872. Which is a direct payoff of `REQUIREMENTS.md` B8.
1873. Server-authoritative merge of habitat deltas (`REQUIREMENTS.md` C6).
1874. Version-gated visits so mismatched builds cannot corrupt state.
1875. Conflict resolution favouring the host, with a visible log.
1876. Save-format migration across the network boundary (`REQUIREMENTS.md` E5).
1877. Bandwidth budget: strokes and soil deltas are small, and that is the point.
1878. Which makes this cheap to run, unlike a real-time world.
1879. Backend scope kept to sync, entitlements and telemetry
      (`EM_BRIEF.md` §2.1).
1880. The game fully playable with all services down, always.
1881. Which means multiplayer is additive, never load-bearing.
1882. Optional real-time co-presence for two players, later and carefully.
1883. Which is a performance and netcode project, not a v1 feature.
1884. Voice and text chat handled by the platform, not by us.
1885. Because owning a chat system means owning a moderation system.
1886. **Player↔player romance remains out** (`PD_BRIEF.md` §7), for the reasons
      stated there, unchanged.
1887. Agents as the romance surface, which is safe by construction.
1888. And which is now *better*, not merely safer, because agents have land.
1889. Player↔player friendship, rivalry and collaboration as the social layer.
1890. Rivalry expressed through stewardship comparison, not combat.
1891. No PvP, no griefing surface, no destructible guest actions.
1892. Which is a deliberate genre choice and should be stated as one.
1893. Neighbourhood norms and light social governance.
1894. Reporting and blocking at the neighbourhood level.
1895. Age-appropriate matchmaking, given the mature content
      (`PD_BRIEF.md` §8).
1896. Archive content never shared between players, under any circumstance.
1897. Which is a hard content boundary, documented.
1898. Multiplayer telemetry on what people actually do socially.
1899. Post-EA funding decision on the Committee, informed by that data
      (`EM_BRIEF.md` §10).
1900. Publish the limits: async only at v1, one habitat per player, no real-time
      co-presence.

## AO. The Committee — competition over finite slots (1901–1950)

*`PD_BRIEF.md` §7's v2, which agents make viable much earlier than expected.*

1901. Four to eight players on one shared habitat, seasonal and opt-in.
1902. A shared agent cast, romanceable by all of them.
1903. **Genuinely finite Crèche slots**, fewer than the number of players.
1904. Which is the competition, and it is material rather than combative.
1905. Slots gated by the habitat's carrying capacity, which players collectively
      determine.
1906. So players can cooperate to raise the ceiling *and* compete for what is
      under it.
1907. Which is the best structural tension available and it is thematically exact.
1908. Agents choosing between suitors on their own criteria (§AJ item 1595).
1909. Criteria legible through their behaviour and land, so competition is
      skill-expressive.
1910. Which means winning is a matter of being genuinely better to farm beside.
1911. Agents who prefer a player who helped them over one who courted them.
1912. Sabotage as a material act: diverting water, felling upstream.
1913. Which is possible, visible, attributable, and socially punished by agents.
1914. Attribution through the chronicle, so blame is evidenced.
1915. False accusation as possible, and resolvable by evidence.
1916. Which is a genuinely novel social mechanic built on the chronicle.
1917. Alliances between players over watershed position.
1918. Upstream players holding real power over downstream ones (§I item 170).
1919. Which makes map position matter enormously and should be rotated.
1920. Water rights negotiation as the season's central politics
      (§O item 330).
1921. Agreements recorded, enforceable socially, breakable.
1922. Reputation with agents as the real scoreboard.
1923. Seasonal structure with a defined end and a settlement.
1924. Which bounds toxicity by bounding duration (`EM_BRIEF.md` §10).
1925. Results carrying forward as prestige, never as power.
1926. No mechanical advantage from a previous season, ever.
1927. Spectating a season in progress.
1928. Post-season chronicle as a shared story of what happened.
1929. Which is the most shareable artefact the game could produce.
1930. Highlight generation from chronicle significance.
1931. Season archetypes: drought season, flood season, cold season.
1932. Which changes the strategy space per season.
1933. Handicapping by position rather than by skill.
1934. Matchmaking on stewardship history rather than on wins.
1935. Which discourages extraction strategies structurally.
1936. Solo queue and party queue kept separate.
1937. Griefing countermeasures: no permanent destruction, host-style undo logs.
1938. Vote-based ejection at neighbourhood scale.
1939. Clear rules about sabotage, stated in fiction.
1940. Agents policing norms among players too (§AI item 1545).
1941. Which offloads much of the moderation onto the fiction, elegantly.
1942. Moderation tooling for the rest, and staffing it as a real cost.
1943. Live-ops load acknowledged honestly (`EM_BRIEF.md` §10's pushback).
1944. Which is why this stays post-EA and behind a season structure.
1945. Telemetry on whether the competition is fun or merely stressful.
1946. Willingness to cut the mode if it is not.
1947. Committee mode never touching the Archive layer.
1948. Which keeps the content boundary intact.
1949. A cooperative variant with no competition, for players who want the shared
      habitat without the contest.
1950. Publish the limits: seasonal, opt-in, no ranked ladder, no permanent
      destruction.

## AP. Safety, moderation and consent (1951–1980)

1951. Consent as a modelled, affirmative state in every intimacy interaction
      (§AJ item 1634).
1952. Refusal always available and never penalised mechanically.
1953. Granular player content controls, honoured everywhere.
1954. Content settings surfaced at first run, not buried.
1955. The Archive layer as a separate optional download (§AL item 1754).
1956. Age verification for that layer only, via a vendor (`PD_BRIEF.md` §8).
1957. No generated text or imagery in the intimacy layer, ever
      (§AK item 1707).
1958. Which is both a safety and a quality decision.
1959. All footage from cast actors with signed likeness releases
      (`PD_BRIEF.md` §8.2).
1960. Recordkeeping as a compliance requirement, budgeted.
1961. No player-uploaded content anywhere in the game.
1962. Which removes an entire moderation category by design.
1963. Notes and blueprints as the only player-authored content, heavily bounded.
1964. Note text filtered and reportable (§AN item 1855).
1965. No free-text between players; platform chat only (§AN item 1884).
1966. Reporting flows that are short and effective.
1967. Blocking at neighbourhood level, immediate and total.
1968. Archive content never transmitted between clients (§AN item 1896).
1969. Local-only storage for it, encrypted at rest.
1970. Clear data policy on what telemetry collects, in plain language.
1971. Opt-out telemetry that genuinely works.
1972. No dark patterns in any progression or social system.
1973. No engagement mechanics that punish absence (§AN item 1866).
1974. Session-length neutrality: short sessions are fully valid.
1975. Accessibility as a gate (§T items 629–631), including reading speed,
      contrast, motion and remappable input — which already exists.
1976. Content warnings that are specific rather than generic.
1977. A documented policy on the depiction of intimacy, published.
1978. External review of that policy before launch.
1979. Willingness to cut anything that cannot be done responsibly.
1980. Publish the limits: the boundaries above are commitments, and this page is
      where they are checked.

## AQ. Architecture for agents and the network (1981–2000)

1981. **Agents in the Rust sim core**, not in GDScript — they are simulation,
      not presentation.
1982. Which keeps them deterministic, headless-testable and engine-independent.
1983. Agent actions routed through the same functions the player's input calls.
1984. A single action queue so player and agent actions interleave identically.
1985. Which makes replay and multiplayer verification tractable.
1986. Per-agent seeded RNG streams (§AG item 1375).
1987. Agent decisions logged for the chronicle and for debugging (§AI item 1558).
1988. Agent tick in the documented order, after fauna and before economy
      (§AG item 1361).
1989. Agent budget in the frame-time table (§V item 752), measured.
1990. Twelve agents at T0 as the tested figure, with the dial documented
      (§AH item 1486).
1991. Relationship state as derived-from-chronicle, cached per frame, never
      stored twice.
1992. Which is the single most important structural decision in this document.
1993. The language layer strictly outside the simulation loop (§AK item 1676).
1994. Intent as a serialisable structure, so dialogue is testable in CI
      (§AK item 1709).
1995. Network deltas as strokes, soil, modules and chronicle entries — nothing
      derived.
1996. Which keeps sync payloads small and verification simple.
1997. Headless colony runs as a first-class test (§AH item 1487).
1998. A colony-survival distribution across seeds as a balance instrument
      (§AH item 1488).
1999. `NEXT.md` still capped at ten. Two thousand items is a **register, not a
      queue**.
2000. And the rule, unchanged across all two thousand: **honesty of mechanism
      beats cosmetic spectacle; legibility beats completeness; delight in the
      first ninety seconds beats feature count; stated limits beat implied
      precision.**

---

## What I would actually work through, in order

| Wave | Items | Why |
|---|---|---|
| **1** | 1401–1409, 1981–1985 | **One agent that farms.** Rust-side, planning through the player's own API, visible in the world. Everything else is downstream of this existing. |
| **2** | 1561–1572, 1991 | **Affinity from the chronicle**, not from points. Get "you dug the channel that saved my field" working as a query. The romance system is then structurally correct forever. |
| **3** | 1491–1502, 1512–1520 | Perception, utility scoring and the traits that make two agents visibly different in how they treat land. |
| **4** | 1651–1662, 1710 | The intent layer with Ink templates. Shipping-quality dialogue with no model, offline. |
| **5** | 1711–1724 | The authored cast as agents, with beats gated on world state and a reachability validator. |
| **6** | 1771–1776, 1783–1788 | The Crèche gated on real carrying capacity — which is what makes fixing the drainage a romance action. |
| **7** | 1821–1841 | Async Shared Habitat: visiting, gifting, and strain exchange with real genetics. |

**The one to build first, and the test of the whole idea:** a single agent who
farms a plot next to yours, whose field visibly reflects how well he works it,
and who says one line — generated from a chronicle query — about something you
actually did. If that lands, this design is right. If it does not, no amount of
dialogue will save it.

**And the sentence the design has been reaching for**, from item 1761: *fixing
the drainage is a romance action.* Not a metaphor. The Crèche needs carrying
capacity, carrying capacity needs arable land, arable land needs water in the
right place — and he will not commit until the colony is secure. Two pillars,
one mechanic, no dialogue tree required.
