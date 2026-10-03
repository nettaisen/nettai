--!strict
-- The patch-cards system (docs/engine/patch-cards.md): as the round is set
-- up (`round_setup`, before anything reads the stats), a player's installed
-- patch cards (the engine's definitions and the player's setup,
-- `battle.patch_cards`; their effects ./cards.luau's) change the navi's
-- stats. The cards are the engine's; what their effects do is BN6's rules'.
--
-- The original (the Japanese `reloadCurNaviStatBoosts`, its card routine)
-- keeps a block of slots, one per kind of effect, seeded from the stats the
-- NaviCust made. Each switched-on card in the list's order runs its effects
-- in the order of their kinds (cards.ORDER, not the card's order), each
-- writing its slot (a later card's write wins); then the slots a card wrote
-- are copied into the stats, some of them through tables; then the bugs are
-- counted. A card that lists one effect twice applies the first only. (Two
-- effects of one kind with different choices, say two charged shots, apply
-- in the card's order here; the original orders them by its effect numbers.
-- No card has two.)

local cards = require("./cards")

-- The bug slots BugStop holds back.
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

local function clamp(v: number, lo: number, hi: number): number
    return if v < lo then lo elseif v > hi then hi else v
end

-- A byte read as the original's `ldrsb` reads it.
local function signed_byte(v: number): number
    v = bit32.band(v, 0xFF)
    return if v >= 0x80 then v - 0x100 else v
end

-- Division toward zero (`Div`, the BIOS's).
local function div(n: number, d: number): number
    local q = n / d
    return if q >= 0 then math.floor(q) else math.ceil(q)
end

-- A card's effects in the order they apply: by kind; of two that are the
-- same effect, the first.
local function in_order(effects: { PatchCardEffect }): { PatchCardEffect }
    local seen: { [any]: boolean } = {}
    type Entry = { e: PatchCardEffect, i: number }
    local list: { Entry } = {}
    for i, e in effects do
        local rank = cards.RANK[e.kind]
        if rank == nil then
            error(string.format("patch card effect %q is no kind of effect", tostring(e.kind)))
        end
        local choice = cards.CHOICE[e.kind]
        local key: any = e.kind
        if choice then
            key = e.kind .. "/" .. tostring((e :: any)[choice])
        end
        if not seen[key] then
            seen[key] = true
            table.insert(list, { e = e, i = i })
        end
    end
    table.sort(list, function(a: Entry, b: Entry): boolean
        local ra, rb = cards.RANK[a.e.kind], cards.RANK[b.e.kind]
        if ra ~= rb then
            return ra < rb
        end
        return a.i < b.i
    end)
    local out = {}
    for _, x in list do
        table.insert(out, x.e)
    end
    return out
end

-- `sub_8142578`'s count: whether the stats have a NaviCust bug (event flag
-- 0x1723, which the emotion window reads with cards installed).
local function bugged(stats: Navi): boolean
    return stats.step_bug == 1
        or stats.panel_trail_level ~= 0
        or stats.buster_blanks ~= 0
        or stats.hit_status ~= 0
        or stats.custom_damage ~= 0
        or stats.emotion_bug ~= 0
        or stats.hp_drain ~= 0
        or stats.custom_drain ~= 0
        or stats.battle_start_bug ~= 0
        or stats.encounters == 1
        or stats.chip_drops == 1
        or stats.support_bug
end

-- `round_setup`: `side`'s switched-on cards change its stats; with cards
-- installed, switched on or not, the console's emotion window glitches if
-- the stats then have a NaviCust bug (the Japanese `sub_813BF1C` reads event
-- flag 0x1723, which the card routine sets, rather than 0x1720). Only
-- MegaMan's stats: with a link navi operated the original doesn't run the
-- routine, and the flag is as the stats are.
local function apply(side: number)
    local installed = battle.patch_cards(side)
    if #installed == 0 then
        return
    end
    local stats = battle.navi(side)
    if stats.navi.forms == nil then
        battle.set_emotion_window_glitch(side, bugged(stats))
        return
    end
    -- The block (`sub_8141B42`): the working HP and the slots, seeded from
    -- the stats; and which slots a card wrote.
    local hp = stats.max_hp
    local slot: { [string]: any } = {
        attack = stats.attack,
        rapid = stats.rapid,
        charge = stats.charge,
        custom = stats.custom_level,
        mega = stats.mega_level,
        giga = stats.giga_level,
        body = bit32.band(stats.element + 4, 0xFF),
    }
    local written: { [string]: boolean } = {}
    local function put(name: string, v: any)
        slot[name] = v
        written[name] = true
    end
    local function stat(name: string, d: number, lo: number, hi: number)
        put(name, bit32.band(clamp(signed_byte(slot[name]) + d, lo, hi), 0xFF))
    end
    for _, c in installed do
        if c.enabled then
            for _, e in in_order(c.card.effects) do
                local k, n = e.kind, e.amount or 0
                if k == "hp_add" then
                    hp = clamp(hp + n * 10, 1, 9999)
                elseif k == "hp_sub" then
                    hp = clamp(hp - n * 10, 1, 9999)
                elseif k == "hp_percent_add" then
                    hp = clamp(div(hp * n + hp * 100, 100), 1, 9999)
                elseif k == "hp_percent_sub" then
                    hp = clamp(div(hp * 100 - hp * n, 100), 1, 9999)
                elseif k == "body" then
                    put("body", cards.BODY[e.element :: string] + 4)
                elseif k == "attack_add" then
                    stat("attack", n, 0, 9)
                elseif k == "attack_sub" then
                    stat("attack", -n, 0, 9)
                elseif k == "attack_times" then
                    put("attack", bit32.band(clamp((signed_byte(slot.attack) + 1) * n - 1, 0, 9), 0xFF))
                elseif k == "rapid_add" then
                    stat("rapid", n, 0, 4)
                elseif k == "rapid_sub" then
                    stat("rapid", -n, 0, 4)
                elseif k == "charge_add" then
                    stat("charge", n, 0, 4)
                elseif k == "charge_sub" then
                    stat("charge", -n, 0, 4)
                elseif k == "custom_add" then
                    stat("custom", n, 2, 8)
                elseif k == "custom_sub" then
                    stat("custom", -n, 2, 8)
                elseif k == "mega_add" then
                    stat("mega", n, 1, 10)
                elseif k == "mega_sub" then
                    stat("mega", -n, 1, 10)
                elseif k == "giga_add" then
                    stat("giga", n, 1, 10)
                elseif k == "giga_sub" then
                    -- (Its handler stores without the clamp.)
                    put("giga", bit32.band(signed_byte(slot.giga) - n, 0xFF))
                elseif k == "b_button" or k == "charged_shot" or k == "back_special" then
                    put(k, e.weapon or false)
                elseif k == "buster_shot" or k == "charge_shot_kind" then
                    put(k, e.variant or false)
                elseif k == "first_barrier" then
                    put(k, e.barrier or false)
                elseif k == "gauge" then
                    put(k, e.gauge)
                elseif k == "panel_trail" then
                    put(k, e)
                elseif k == "buster_blanks" then
                    put(k, e)
                elseif k == "charge_blanks" then
                    put(k, e.charged)
                elseif k == "hit_status" then
                    put(k, cards.HIT_STATUS[e.status :: string])
                elseif e.on ~= nil then
                    put(k, e.on)
                else
                    put(k, n)
                end
            end
        end
    end

    -- The copy (`sub_8142230`): the HP, then each slot a card wrote.
    stats.max_hp = hp
    -- (The reload then sets the HP: in the real world to the maximum, on
    -- the Internet to the lesser of the two. A netbattle's comes from the
    -- real world's.)
    stats.hp = hp
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
    if copy("charge_blanks") then stats.buster_charged = slot.charge_blanks end
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
    -- (BugStop's own byte, NaviStats+0x1F, only the NaviCust's bug compile
    -- reads.)
    if copy("chip_shuffle") then stats.chip_shuffle = slot.chip_shuffle end
    -- ChpShufl from a card turns NumbrOpn off (the original tests the
    -- block the last application left: the same cards').
    if written.chip_shuffle and slot.chip_shuffle then
        stats.number_open = false
    end
    battle.set_emotion_window_glitch(side, bugged(stats))
end

return define.system {
    id = "bn6:patch-cards",
    hooks = { round_setup = apply },
}
