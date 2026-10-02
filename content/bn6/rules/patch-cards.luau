--!strict
-- Patch cards (the Japanese games' Modification Cards, 改造カード;
-- docs/engine/patch-cards.md): the effects a card definition lists, and
-- their application to a navi's stats at the round's start (the role
-- `hooks.patch_cards`).
--
-- The original (the Japanese `reloadCurNaviStatBoosts`, its card routine)
-- keeps a block of slots, one per kind of effect, seeded from the stats the
-- NaviCust made. Each switched-on card in the list's order runs its effects
-- in the order of their kinds below (not the card's order), each writing its
-- slot (a later card's write wins); then the slots a card wrote are copied
-- into the stats, some of them through tables; then the bugs are counted.
-- A card that lists one effect twice applies the first only. (Two effects
-- of one kind with different choices, say two charged shots, apply in the
-- card's order here; the original orders them by its effect numbers. No
-- card has two.)

local patch_cards = {}

export type Effect = PatchCardEffect

-- The kinds in the order they apply (the original's effect numbers).
local ORDER: { string } = {
    "hp_add",
    "hp_percent_add",
    "hp_sub",
    "hp_percent_sub",
    "body",
    "attack_add",
    "attack_sub",
    "attack_times",
    "rapid_add",
    "rapid_sub",
    "charge_add",
    "charge_sub",
    "custom_add",
    "custom_sub",
    "mega_add",
    "mega_sub",
    "giga_add",
    "giga_sub",
    "super_armor",
    "status_guard",
    "float_shoes",
    "air_shoes",
    "undershirt",
    "b_button",
    "buster_shot",
    "charged_shot",
    "charge_shot_kind",
    "first_barrier",
    "back_special",
    "chip_recovery",
    "gauge",
    "rush",
    "beat",
    "tango",
    "step_bug",
    "panel_trail",
    "buster_blanks",
    "charge_blanks",
    "hit_status",
    "custom_damage",
    "emotion_bug",
    "hp_drain",
    "custom_hp_drain",
    "status_bug",
    "bug_stopper",
    "chip_shuffle",
}
local RANK: { [string]: number } = {}
for i, kind in ORDER do
    RANK[kind] = i
end

-- The kinds whose choice is part of what the effect is (the original has
-- an effect number per choice), and the choice's field.
local CHOICE: { [string]: string } = {
    body = "element",
    b_button = "weapon",
    buster_shot = "variant",
    charged_shot = "weapon",
    charge_shot_kind = "variant",
    first_barrier = "barrier",
    back_special = "weapon",
    gauge = "gauge",
    panel_trail = "trail",
    hit_status = "status",
}

-- The element bytes of the bodies (NaviStats+0x10).
local BODY: { [string]: number } = { null = 0, fire = 1, aqua = 2, elec = 3, wood = 4 }
-- What a step leaves behind, by the panel the effect names (the trail
-- byte, NaviStats+0x12: `byte_814291C`'s rows; water and magnet panels,
-- which BN6 doesn't have, leave the normal panel's 2).
local TRAILS: { [string]: number } = {
    crack = 3,
    hole = 1,
    water = 2,
    ice = 7,
    grass = 6,
    magnet = 2,
    holy = 5,
    poison = 4,
    normal = 2,
    none = 0,
}
-- The status a hit gives (NaviStats+0x16).
local HIT_STATUS: { [string]: number } = { blind = 1, confuse = 2, hp_bug = 3, none = 0 }

local function effect(e: Effect): Effect
    return e
end

local function whole(n: number, what: string, max: number): number
    if n ~= math.floor(n) or n < 0 or n > max then
        error(string.format("patch card effect %s: %s is not a whole number from 0 to %d", what, tostring(n), max))
    end
    return n
end

-- ---- The effects ---------------------------------------------------------

-- An effect the card shows as a bug (red on its menu; the application
-- doesn't care).
function patch_cards.bug(e: Effect): Effect
    local copy = table.clone(e)
    copy.bug = true
    return effect(copy)
end

-- Max HP: plus or minus `n` (a multiple of 10).
function patch_cards.hp(n: number): Effect
    if n % 10 ~= 0 then
        error(string.format("patch card HP %d: the cards change HP by tens", n))
    end
    local kind = if n >= 0 then "hp_add" else "hp_sub"
    return effect({ kind = kind :: any, amount = whole(math.abs(n) / 10, "hp", 255) })
end

-- Max HP: plus or minus `n` percent.
function patch_cards.hp_percent(n: number): Effect
    local kind = if n >= 0 then "hp_percent_add" else "hp_percent_sub"
    return effect({ kind = kind :: any, amount = whole(math.abs(n), "hp_percent", 255) })
end

-- MegaMan's element.
function patch_cards.body(element: ElementName): Effect
    if BODY[element] == nil then
        error(string.format("patch card body %q is no element", element))
    end
    return effect({ kind = "body", element = element })
end

-- The buster's Attack, Rapid and Charge, the custom screen's size, the
-- folder's Mega and Giga limits: plus or minus `n`.
local function signed(add: string, sub: string, what: string): (number) -> Effect
    return function(n: number): Effect
        local kind = if n >= 0 then add else sub
        return effect({ kind = kind :: any, amount = whole(math.abs(n), what, 255) })
    end
end
patch_cards.attack = signed("attack_add", "attack_sub", "attack")
patch_cards.rapid = signed("rapid_add", "rapid_sub", "rapid")
patch_cards.charge = signed("charge_add", "charge_sub", "charge")
patch_cards.custom = signed("custom_add", "custom_sub", "custom")
patch_cards.mega_folder = signed("mega_add", "mega_sub", "mega_folder")
patch_cards.giga_folder = signed("giga_add", "giga_sub", "giga_folder")

-- Attack times `n`: (Attack + 1) * n - 1.
function patch_cards.attack_times(n: number): Effect
    return effect({ kind = "attack_times", amount = whole(n, "attack_times", 255) })
end

-- The NaviCust's abilities, given (true) or taken away (false).
local function ability(kind: string): (boolean) -> Effect
    return function(on: boolean): Effect
        return effect({ kind = kind :: any, on = on })
    end
end
patch_cards.super_armor = ability("super_armor")
patch_cards.status_guard = ability("status_guard")
patch_cards.float_shoes = ability("float_shoes")
patch_cards.air_shoes = ability("air_shoes")
patch_cards.undershirt = ability("undershirt")
patch_cards.rush = ability("rush")
patch_cards.beat = ability("beat")
patch_cards.tango = ability("tango")
patch_cards.chip_shuffle = ability("chip_shuffle")

-- The B button's weapon.
function patch_cards.b_button(weapon: Weapon): Effect
    return effect({ kind = "b_button", weapon = weapon })
end

-- The buster shot's program (nil: none).
function patch_cards.buster_shot(variant: ProjectileVariant?): Effect
    return effect({ kind = "buster_shot", variant = variant })
end

-- The charged shot's weapon (nil: none).
function patch_cards.charged_shot(weapon: Weapon?): Effect
    return effect({ kind = "charged_shot", weapon = weapon })
end

-- The charged shot's program.
function patch_cards.charge_shot_kind(variant: ProjectileVariant): Effect
    return effect({ kind = "charge_shot_kind", variant = variant })
end

-- The barrier the navi enters with (nil: none).
function patch_cards.first_barrier(barrier: Barrier?): Effect
    return effect({ kind = "first_barrier", barrier = barrier })
end

-- The B+Back special's weapon (nil: none).
function patch_cards.back_special(weapon: Weapon?): Effect
    return effect({ kind = "back_special", weapon = weapon })
end

-- HP healed per chip used.
function patch_cards.chip_recovery(n: number): Effect
    return effect({ kind = "chip_recovery", amount = whole(n, "chip_recovery", 255) })
end

-- The custom gauge's speed.
function patch_cards.gauge(speed: "fast" | "slow" | "normal"): Effect
    return effect({ kind = "gauge", gauge = speed })
end

-- NaviCust bugs (a level of 0 fixes the NaviCust's own):
-- steps go astray (1), or not (0);
function patch_cards.step_bug(n: number): Effect
    return effect({ kind = "step_bug", amount = whole(n, "step_bug", 255) })
end

-- a step leaves `panel` behind, `level` of the time;
function patch_cards.panel_trail(panel: string, level: number): Effect
    if TRAILS[panel] == nil then
        error(string.format("patch card panel trail %q is no panel (crack, hole, water, ice, grass, magnet, holy, poison, normal, none)", panel))
    end
    return effect({ kind = "panel_trail", trail = panel, level = whole(level, "panel_trail", 15) })
end

-- the buster fires `blanks` blank shots and `charged` charged ones in 16;
function patch_cards.buster_blanks(blanks: number, charged: number): Effect
    return effect({ kind = "buster_blanks", blanks = whole(blanks, "buster_blanks", 15), charged = whole(charged, "buster_blanks", 15) })
end

-- the charged shots of the buster's 16 (no card has it);
function patch_cards.charge_blanks(charged: number): Effect
    return effect({ kind = "charge_blanks", charged = whole(charged, "charge_blanks", 15) })
end

-- a hit gives a status;
function patch_cards.hit_status(status: "blind" | "confuse" | "hp_bug" | "none"): Effect
    return effect({ kind = "hit_status", status = status })
end

-- the custom screen does `n` damage as it opens;
function patch_cards.custom_damage(n: number): Effect
    return effect({ kind = "custom_damage", amount = whole(n, "custom_damage", 255) })
end

-- the emotion swings;
function patch_cards.emotion_bug(n: number): Effect
    return effect({ kind = "emotion_bug", amount = whole(n, "emotion_bug", 255) })
end

-- the HP drain in the fight and on the custom screen go up `n` levels (to
-- 7), or away (0);
function patch_cards.hp_drain(n: number): Effect
    return effect({ kind = "hp_drain", amount = whole(n, "hp_drain", 255) })
end
function patch_cards.custom_hp_drain(n: number): Effect
    return effect({ kind = "custom_hp_drain", amount = whole(n, "custom_hp_drain", 255) })
end

-- the battle start's bug (1, or worse past 1);
function patch_cards.status_bug(n: number): Effect
    return effect({ kind = "status_bug", amount = whole(n, "status_bug", 255) })
end

-- BugStop: at 1, none of the cards' bugs above is copied (no card has it).
function patch_cards.bug_stopper(n: number): Effect
    return effect({ kind = "bug_stopper", amount = whole(n, "bug_stopper", 255) })
end

-- ---- The application -----------------------------------------------------

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
local function in_order(effects: { Effect }): { Effect }
    local seen: { [any]: boolean } = {}
    local list: { { e: Effect, i: number } } = {}
    for i, e in effects do
        local rank = RANK[e.kind]
        if rank == nil then
            error(string.format("patch card effect %q is no kind of effect", tostring(e.kind)))
        end
        local choice = CHOICE[e.kind]
        local key: any = e.kind
        if choice then
            key = e.kind .. "/" .. tostring((e :: any)[choice])
        end
        if not seen[key] then
            seen[key] = true
            table.insert(list, { e = e, i = i })
        end
    end
    table.sort(list, function(a, b)
        local ra, rb = RANK[a.e.kind], RANK[b.e.kind]
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

-- The role `hooks.patch_cards`: `side`'s cards change its stats; whether the
-- stats then have a NaviCust bug. Only MegaMan's: with a link navi
-- operated the original doesn't run the routine.
function patch_cards.apply(side: number): boolean
    local stats = battle.navi(side)
    if stats.navi.forms == nil then
        return bugged(stats)
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
    for _, installed in battle.patch_cards(side) do
        if installed.enabled then
            for _, e in in_order(installed.card.effects) do
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
                    put("body", BODY[e.element :: string] + 4)
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
                    put(k, HIT_STATUS[e.status :: string])
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
        stats.panel_trail = TRAILS[slot.panel_trail.trail]
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
    return bugged(stats)
end

return patch_cards
