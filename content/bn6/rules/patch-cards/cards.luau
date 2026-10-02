--!strict
-- Patch cards (the Japanese games' Modification Cards, 改造カード;
-- docs/engine/patch-cards.md): what a card is. A card is a record of type
-- "patch-card" (`patch_cards.card`), its effects values this module's
-- constructors make; the patch-cards system (./system.luau) applies a
-- player's installed cards to the navi's stats as the round is set up.

local patch_cards = {}

export type Effect = PatchCardEffect

-- The kinds in the order they apply (the original's effect numbers).
patch_cards.ORDER = {
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
} :: { string }
patch_cards.RANK = {} :: { [string]: number }
for i, kind in patch_cards.ORDER do
    patch_cards.RANK[kind] = i
end

-- The kinds whose choice is part of what the effect is (the original has
-- an effect number per choice), and the choice's field.
patch_cards.CHOICE = {
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
} :: { [string]: string }

-- The element bytes of the bodies (NaviStats+0x10).
patch_cards.BODY = { null = 0, fire = 1, aqua = 2, elec = 3, wood = 4 } :: { [string]: number }
-- What a step leaves behind, by the panel the effect names (the trail
-- byte, NaviStats+0x12: `byte_814291C`'s rows; water and magnet panels,
-- which BN6 doesn't have, leave the normal panel's 2).
patch_cards.TRAILS = {
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
} :: { [string]: number }
-- The status a hit gives (NaviStats+0x16).
patch_cards.HIT_STATUS = { blind = 1, confuse = 2, hp_bug = 3, none = 0 } :: { [string]: number }

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
    if patch_cards.BODY[element] == nil then
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
    if patch_cards.TRAILS[panel] == nil then
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

-- ---- The card ------------------------------------------------------------

-- A patch card: its key ("patch-card/<name>"), the name its menu shows, its
-- MB (the installed cards may take 80) and its effects.
function patch_cards.card(spec: PatchCard): PatchCard
    if string.sub(spec.id, 1, #"patch-card/") ~= "patch-card/" then
        error(string.format("patch card %q: a card's key starts with patch-card/", spec.id))
    end
    whole(spec.mb, spec.id .. "'s mb", 80)
    for _, e in spec.effects do
        if patch_cards.RANK[e.kind] == nil then
            error(string.format("patch card %s: effect %q is no kind of effect", spec.id, tostring(e.kind)))
        end
    end
    return define.record("patch-card", spec)
end

return patch_cards
