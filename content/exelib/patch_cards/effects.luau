--!strict
-- Patch cards' effects as EXE5's and EXE6's cards share them (docs/engine/patch-cards.md; EXE5's, exe5-map.md §15.14):
-- a card is the engine's (`define.patch_card`, a game's cards/), its effects the values these constructors make,
-- which a game's patch cards part applies to the navi's stats as the round is set up (@exelib/patch_cards/apply).
-- Each game's rules/patch_cards/cards.luau gives these and its own tables (the kinds' order, the trails' panels).

local effects = {}

-- The panels a step can leave behind (a game's trail table gives each its byte).
local TRAILS = {
    crack = true,
    hole = true,
    water = true,
    ice = true,
    grass = true,
    magnet = true,
    holy = true,
    poison = true,
    normal = true,
    none = true,
} :: { [string]: boolean }
-- The elements of the bodies.
local BODIES = { null = true, fire = true, aqua = true, elec = true, wood = true } :: { [string]: boolean }

local function effect(e: PatchCardEffect): PatchCardEffect
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
function effects.bug(e: PatchCardEffect): PatchCardEffect
    local copy = table.clone(e)
    copy.bug = true
    return effect(copy)
end

-- Max HP: plus or minus `n` (a multiple of 10).
function effects.hp(n: number): PatchCardEffect
    if n % 10 ~= 0 then
        error(string.format("patch card HP %d: the cards change HP by tens", n))
    end
    local kind = if n >= 0 then "hp_add" else "hp_sub"
    return effect({ kind = kind :: any, amount = whole(math.abs(n) / 10, "hp", 255) })
end

-- Max HP: plus or minus `n` percent.
function effects.hp_percent(n: number): PatchCardEffect
    local kind = if n >= 0 then "hp_percent_add" else "hp_percent_sub"
    return effect({ kind = kind :: any, amount = whole(math.abs(n), "hp_percent", 255) })
end

-- MegaMan's element.
function effects.body(element: ElementName): PatchCardEffect
    if not BODIES[element] then
        error(string.format("patch card body %q is no element", element))
    end
    return effect({ kind = "body", element = element })
end

-- The buster's Attack, Rapid and Charge, the custom screen's size, the
-- folder's Mega and Giga limits, EXE5's soul turns: plus or minus `n`.
local function signed(add: string, sub: string, what: string): (number) -> PatchCardEffect
    return function(n: number): PatchCardEffect
        local kind = if n >= 0 then add else sub
        return effect({ kind = kind :: any, amount = whole(math.abs(n), what, 255) })
    end
end
effects.attack = signed("attack_add", "attack_sub", "attack")
effects.rapid = signed("rapid_add", "rapid_sub", "rapid")
effects.charge = signed("charge_add", "charge_sub", "charge")
effects.custom = signed("custom_add", "custom_sub", "custom")
effects.mega_folder = signed("mega_add", "mega_sub", "mega_folder")
effects.giga_folder = signed("giga_add", "giga_sub", "giga_folder")
effects.soul_time = signed("soul_time_add", "soul_time_sub", "soul_time")

-- Attack times `n`: (Attack + 1) * n - 1.
function effects.attack_times(n: number): PatchCardEffect
    return effect({ kind = "attack_times", amount = whole(n, "attack_times", 255) })
end

-- The NaviCust's abilities, given (true) or taken away (false).
local function ability(kind: string): (boolean) -> PatchCardEffect
    return function(on: boolean): PatchCardEffect
        return effect({ kind = kind :: any, on = on })
    end
end
effects.super_armor = ability("super_armor")
effects.status_guard = ability("status_guard")
effects.float_shoes = ability("float_shoes")
effects.air_shoes = ability("air_shoes")
effects.undershirt = ability("undershirt")
effects.rush = ability("rush")
effects.beat = ability("beat")
effects.tango = ability("tango")
effects.chip_shuffle = ability("chip_shuffle")
-- EXE5's programs that work outside battle (a NaviStats byte no battle reads): SneakRun, Millions, AutoHeal,
-- MegVirus.
effects.sneak_run = ability("sneak_run")
effects.millions = ability("millions")
effects.auto_heal = ability("auto_heal")
effects.mega_virus = ability("mega_virus")

-- EXE5's random encounters' element (OilBody, Fish, Battery, Jungle, Search: a NaviStats byte no battle reads).
function effects.encounter_element(what: "oil_body" | "fish" | "battery" | "jungle" | "search"): PatchCardEffect
    return effect({ kind = "encounter_element", what = what })
end

-- The B button's weapon.
function effects.b_button(weapon: Weapon): PatchCardEffect
    return effect({ kind = "b_button", weapon = weapon })
end

-- The buster shot's program (nil: none).
function effects.buster_shot(variant: ProjectileVariant?): PatchCardEffect
    return effect({ kind = "buster_shot", variant = variant })
end

-- The charged shot's weapon (nil: none).
function effects.charged_shot(weapon: Weapon?): PatchCardEffect
    return effect({ kind = "charged_shot", weapon = weapon })
end

-- The charged shot's program (nil: none).
function effects.charge_shot_kind(variant: ProjectileVariant?): PatchCardEffect
    return effect({ kind = "charge_shot_kind", variant = variant })
end

-- The barrier the navi enters with (nil: none).
function effects.first_barrier(barrier: Barrier?): PatchCardEffect
    return effect({ kind = "first_barrier", barrier = barrier })
end

-- The B+Back special's weapon (nil: none).
function effects.back_special(weapon: Weapon?): PatchCardEffect
    return effect({ kind = "back_special", weapon = weapon })
end

-- HP healed per chip used.
function effects.chip_recovery(n: number): PatchCardEffect
    return effect({ kind = "chip_recovery", amount = whole(n, "chip_recovery", 255) })
end

-- The custom gauge's speed.
function effects.gauge(speed: "fast" | "slow" | "normal"): PatchCardEffect
    return effect({ kind = "gauge", gauge = speed })
end

-- NaviCust bugs (a level of 0 fixes the NaviCust's own):
-- steps go astray (1), or not (0);
function effects.step_bug(n: number): PatchCardEffect
    return effect({ kind = "step_bug", amount = whole(n, "step_bug", 255) })
end

-- a step leaves `panel` behind, `level` of the time;
function effects.panel_trail(panel: string, level: number): PatchCardEffect
    if not TRAILS[panel] then
        error(string.format("patch card panel trail %q is no panel (crack, hole, water, ice, grass, magnet, holy, poison, normal, none)", panel))
    end
    return effect({ kind = "panel_trail", trail = panel, level = whole(level, "panel_trail", 15) })
end

-- the buster fires `blanks` blank shots and `charged` charged ones in 16;
function effects.buster_blanks(blanks: number, charged: number): PatchCardEffect
    return effect({ kind = "buster_blanks", blanks = whole(blanks, "buster_blanks", 15), charged = whole(charged, "buster_blanks", 15) })
end

-- the charged shots of the buster's 16 (EXE6's; no card has it);
function effects.charge_blanks(charged: number): PatchCardEffect
    return effect({ kind = "charge_blanks", charged = whole(charged, "charge_blanks", 15) })
end

-- a hit gives a status;
function effects.hit_status(status: "blind" | "confuse" | "hp_bug" | "none"): PatchCardEffect
    return effect({ kind = "hit_status", status = status })
end

-- the custom screen does `n` damage as it opens;
function effects.custom_damage(n: number): PatchCardEffect
    return effect({ kind = "custom_damage", amount = whole(n, "custom_damage", 255) })
end

-- the emotion swings;
function effects.emotion_bug(n: number): PatchCardEffect
    return effect({ kind = "emotion_bug", amount = whole(n, "emotion_bug", 255) })
end

-- the HP drain in the fight and on the custom screen go up `n` levels (to
-- 7), or away (0);
function effects.hp_drain(n: number): PatchCardEffect
    return effect({ kind = "hp_drain", amount = whole(n, "hp_drain", 255) })
end
function effects.custom_hp_drain(n: number): PatchCardEffect
    return effect({ kind = "custom_hp_drain", amount = whole(n, "custom_hp_drain", 255) })
end

-- the battle start's bug (1, or worse past 1);
function effects.status_bug(n: number): PatchCardEffect
    return effect({ kind = "status_bug", amount = whole(n, "status_bug", 255) })
end

-- BugStop: at 1, none of the cards' bugs above is copied (EXE6: no card has
-- it); EXE5's NaviCust reads it too.
function effects.bug_stopper(n: number): PatchCardEffect
    return effect({ kind = "bug_stopper", amount = whole(n, "bug_stopper", 255) })
end

-- EXE5's card 111 (Bass-Cross MegaMan) switched on: Hub Style (NaviStats +0x4C), `n` as its version's routine writes
-- it (Team ProtoMan's 1, Team Colonel's 2). Not one of the card table's effects: the cards' routine knows the card by
-- its number (0x08137A58).
function effects.hub_style(n: number): PatchCardEffect
    return effect({ kind = "hub_style", amount = whole(n, "hub_style", 255) })
end

return effects
