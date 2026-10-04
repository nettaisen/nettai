--!strict
-- BN5's patch-cards system (docs/design/bn5-map.md §15.14): as the round is set up (`round_setup`, after the
-- NaviCust's compile and before anything else reads the stats), a player's installed patch cards (the engine's
-- definitions and the player's setup, `battle.patch_cards`; their effects ./cards.luau's) change MegaMan's stats,
-- as the original's reload does after the compile (0x0813F97C: the compile, 0x0813FA10; then the cards, 0x08138214;
-- then the HP).
--
-- The cards (0x08138214): Hub Style (NaviStats +0x4C) cleared, and set with card 111 switched on (0x08137A58: 1,
-- Team Colonel's routine 2, 0x081382FC); the
-- block of slots seeded from the stats (0x08137B56) and each switched-on card's effects run over it in the kinds'
-- order (0x08137B00; the run BN6's shares, @common/patch-cards/apply); the HP the working HP; then the slots a card
-- wrote copied into the stats (0x081382B8's 39 routines), some of them through tables, BugStop (the card's, at 1)
-- holding back the bug slots. Their bug count (0x081384D8: the NaviCust's bug bytes, the encounters' and drops' bugs,
-- no supports) sets event flag 0x10C4, the emotion window's glitch when the save has cards (0x0813F650: the card
-- count, not the switched-on cards; without cards the NaviCust's flag, 0x10C1).
--
-- The HP (0x0813F97C, after the cards): in the real world the maximum; in the cyberworld (the setup's `cyberworld`:
-- the save's area, 0x02002944, from 0x80, or event flag 0x10B2) no more than it.
--
-- A side neither of whose NaviCust and cards the setup gives (a recording's: its stats are the original's after all
-- this) is left as it is.

local apply_cards = require("@common/patch-cards/apply")
local cards = require("./cards")

-- What the player's save brings (the navicust system's too: one fact): whether it is in the cyberworld.
type Setup = { cyberworld: boolean }

-- The bug slots BugStop holds back (the copy's flags 0x71 to 0x79).
local BUG_KINDS: { [string]: boolean } = {
    step_bug = true,
    panel_trail = true,
    buster_blanks = true,
    hit_status = true,
    custom_damage = true,
    emotion_bug = true,
    hp_drain = true,
    custom_hp_drain = true,
    status_bug = true,
}

-- 0x081384D8: how many of the bugs the stats have (the cards' flag 0x10C4 from 1).
local function bug_count(stats: Navi): number
    local n = 0
    for _, bugged in {
        stats.step_bug == 1,
        stats.panel_trail_level ~= 0,
        stats.buster_blanks ~= 0,
        stats.hit_status ~= 0,
        stats.custom_damage ~= 0,
        stats.emotion_bug ~= 0,
        stats.hp_drain ~= 0,
        stats.custom_drain ~= 0,
        stats.battle_start_bug ~= 0,
        stats.encounters == 1,
        stats.chip_drops == 1,
        stats.support_bug,
    } do
        if bugged then
            n += 1
        end
    end
    return n
end

-- The cards' run (0x08137B00) over `stats`: the copy into them (0x081382B8's routines).
local function run(stats: Navi, installed: { { card: PatchCard, enabled: boolean } })
    -- The block (0x08137B56): the working HP and the slots, seeded from the stats.
    local slot: { [string]: any } = {
        attack = stats.attack,
        rapid = stats.rapid,
        charge = stats.charge,
        custom = stats.custom_level,
        mega = stats.mega_level,
        giga = stats.giga_level,
        soul_time = bit32.band(stats.soul_turn_bonus, 0xFF),
        body = bit32.band(stats.element + 4, 0xFF),
    }
    local hp, written = apply_cards.run(installed, cards.GAME, stats.max_hp, slot)
    stats.max_hp = hp
    local bug_stop = written.bug_stopper and slot.bug_stopper == 1
    local function copy(name: string): boolean
        return written[name] and not (bug_stop and BUG_KINDS[name])
    end
    if copy("attack") then stats.attack = slot.attack end
    if copy("rapid") then stats.rapid = slot.rapid end
    if copy("charge") then stats.charge = slot.charge end
    if copy("custom") then stats.custom_level = slot.custom end
    if copy("mega") then stats.mega_level = slot.mega end
    if copy("giga") then stats.giga_level = slot.giga end
    if copy("soul_time") then stats.soul_turn_bonus = apply_cards.signed_byte(slot.soul_time) end
    if copy("body") then stats.element = bit32.band(slot.body - 4, 0xFF) end
    if copy("super_armor") then stats.super_armor = slot.super_armor end
    if copy("status_guard") then stats.status_guard = slot.status_guard end
    if copy("float_shoes") then stats.float_shoes = slot.float_shoes end
    if copy("air_shoes") then stats.air_shoes = slot.air_shoes end
    if copy("undershirt") then stats.undershirt = slot.undershirt end
    if copy("b_button") then stats.buster_weapon = slot.b_button or nil end
    if copy("buster_shot") then stats.buster_shot = slot.buster_shot or nil end
    if copy("charged_shot") then stats.charge_shot_weapon = slot.charged_shot or nil end
    if copy("charge_shot_kind") then stats.charge_shot_kind = slot.charge_shot_kind or nil end
    if copy("first_barrier") then stats.first_barrier = slot.first_barrier or nil end
    if copy("back_special") then stats.back_special_weapon = slot.back_special or nil end
    if copy("chip_recovery") then stats.chip_recovery = slot.chip_recovery end
    if copy("gauge") then stats.gauge = slot.gauge end
    if copy("rush") then stats.rush = slot.rush end
    if copy("beat") then stats.beat = slot.beat end
    if copy("tango") then stats.tango = slot.tango end
    -- (SneakRun, +0x1E; the encounters' element, +0x27, ./cards ENCOUNTER; Millions, +0x33; AutoHeal, +0x36;
    -- MegVirus, +0x37: bytes no battle reads.)
    if copy("step_bug") then stats.step_bug = slot.step_bug end
    if copy("panel_trail") then
        stats.panel_trail = cards.TRAILS[slot.panel_trail.trail]
        stats.panel_trail_level = slot.panel_trail.level
    end
    if copy("buster_blanks") then
        stats.buster_blanks = slot.buster_blanks.blanks
        stats.buster_charged = slot.buster_blanks.charged
    end
    if copy("hit_status") then stats.hit_status = slot.hit_status end
    if copy("custom_damage") then stats.custom_damage = slot.custom_damage end
    if copy("emotion_bug") then stats.emotion_bug = slot.emotion_bug end
    if copy("hp_drain") then
        stats.hp_drain = if slot.hp_drain == 0 then 0 else math.min(stats.hp_drain + slot.hp_drain, 7)
    end
    if copy("custom_hp_drain") then
        stats.custom_drain = if slot.custom_hp_drain == 0 then 0 else math.min(stats.custom_drain + slot.custom_hp_drain, 7)
    end
    if copy("status_bug") then
        stats.battle_start_bug = if slot.status_bug == 0 then 0 elseif slot.status_bug == 1 then 9 else 10
    end
    -- (BugStop's own byte, NaviStats +0x1F, only the NaviCust's bug compile reads: the navicust system asks the
    -- cards, ../navicust/system.)
    -- 0x08137A58: card 111 switched on (its version's value).
    stats.hub_style = if written.hub_style then slot.hub_style else 0
end

-- `round_setup`: `side`'s switched-on cards change MegaMan's stats, then the HP.
local function apply(side: number)
    local stats = battle.navi(side)
    if stats.navi.forms == nil then
        return
    end
    local installed = battle.patch_cards(side)
    if battle.navicust(side) == nil and #installed == 0 then
        return
    end
    if #installed > 0 then
        run(stats, installed)
        -- 0x081382A2: the cards' bug flag, which the emotion window reads with cards in the list.
        battle.set_emotion_window_glitch(side, bug_count(stats) > 0)
    else
        stats.hub_style = 0
    end
    -- 0x0813F97C's HP.
    if (system.setup() :: Setup).cyberworld then
        stats.hp = math.min(stats.hp, stats.max_hp)
    else
        stats.hp = stats.max_hp
    end
end

return define.system {
    id = "bn5:patch-cards",
    setup = { cyberworld = "bool" },
    hooks = { round_setup = apply },
}
