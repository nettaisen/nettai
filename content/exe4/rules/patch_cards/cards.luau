--!strict
-- EXE4's patch cards (改造カード: docs/design/exe4-map.md §8): what each card does, the constructors the cards' modules
-- call (content/exe4/patch_cards, written by tools/exe4/gen_patch_cards.py from the cards' handlers, the table at
-- 0x08041E8C) and what each effect does to the stats (`apply`); ./init.luau runs a side's cards in the reload. A card
-- is a `PatchCard` (@exelib's type: EXE4's have no MB, a card a slot) of these effects.

local cards = {}

local function effect(e: PatchCardEffect): PatchCardEffect
    return e
end

-- The HP a card adds (the toolkit's +0x6C, the programs' HP's sum).
function cards.hp(n: number): PatchCardEffect
    return effect { kind = "hp", amount = n }
end

-- The buster's attack set (NaviStats +0x05).
function cards.buster_attack(n: number): PatchCardEffect
    return effect { kind = "buster_attack", amount = n }
end

-- The custom level raised by `n`, at most 8 (+0x12); the Mega and Giga chip limits by `n`, at most 10 (+0x13,
-- +0x14).
function cards.custom(n: number): PatchCardEffect
    return effect { kind = "custom", amount = n }
end
function cards.mega_folder(n: number): PatchCardEffect
    return effect { kind = "mega_folder", amount = n }
end
function cards.giga_folder(n: number): PatchCardEffect
    return effect { kind = "giga_folder", amount = n }
end

-- The custom level lowered by `n`, at least 2 (0x08042A18, 0x08042A38; their count of bug group 11, whose routines
-- do nothing, left out).
function cards.custom_down(n: number): PatchCardEffect
    return effect { kind = "custom_down", amount = n }
end

-- The aura the navi starts with (+0x21).
function cards.aura(b: Barrier): PatchCardEffect
    return effect { kind = "aura", barrier = b }
end

-- The B button's weapon (+0x09), the charged shot's (+0x0A), B+Left's (+0x0C).
function cards.b_button(w: Weapon): PatchCardEffect
    return effect { kind = "b_button", weapon = w }
end
function cards.charged_shot(w: Weapon): PatchCardEffect
    return effect { kind = "charged_shot", weapon = w }
end
function cards.b_left(w: Weapon): PatchCardEffect
    return effect { kind = "b_left", weapon = w }
end

-- The Full Synchro at the start (+0x1F: the rules' stat `full_synchro_start`, which the starting mood reads).
function cards.full_synchro(): PatchCardEffect
    return effect { kind = "full_synchro" }
end

-- MegaMan's color (+0x27: the rules' stat `color`, which his palette reads).
function cards.color(n: number): PatchCardEffect
    return effect { kind = "color", amount = n }
end

-- The panel a step leaves (+0x1B: EXE4's panel type, 1 broken; `effects.panel_trail`).
function cards.panel_trail(panel: number): PatchCardEffect
    return effect { kind = "panel_trail", amount = panel }
end

-- A pair's half (Charge FullCustom's, 103 and 104; Triple Supporter's and All Guard's): `effects` apply only while
-- the card `card` (its key), the other half, sits in slot `slot` (the slots, the toolkit's +0x70) and the stat the
-- first sets doesn't hold its value yet (0x08042758: each half's handler tests both, so of the two halves the one
-- that runs first does it all, its bugs and custom changes too).
function cards.pair(slot: number, card: string, effects: { PatchCardEffect }): PatchCardEffect
    return effect { kind = "pair", pair = { slot = slot, card = card, effects = effects } }
end

-- Whether `stats` already holds what effect `e` sets (a pair's test of its first effect).
function cards.holds(stats: Navi, e: PatchCardEffect): boolean
    if e.kind == "charged_shot" then
        return stats.charge_shot_weapon == e.weapon
    end
    error(string.format("patch card effect %q: no pair tests it", tostring(e.kind)))
end

-- A bug's count raised (0x080476E0), or cleared (0x080476EC): the bug by its name (`what`).
function cards.bug(bug: string, n: number): PatchCardEffect
    return effect { kind = "bug", what = bug, amount = n }
end
function cards.clear_bug(bug: string): PatchCardEffect
    return effect { kind = "clear_bug", what = bug }
end

-- What effect `e` does to `stats`, the work and the counts.
function cards.apply(stats: Navi, e: PatchCardEffect, work: PatchCardWork, counts: PatchCardCounts)
    local k = e.kind
    local n = e.amount or 0
    if k == "hp" then
        work.hp += n
    elseif k == "buster_attack" then
        stats.attack = n
    elseif k == "custom" then
        stats.custom_level = math.min(stats.custom_level + n, 8)
    elseif k == "mega_folder" then
        stats.mega_level = math.min(stats.mega_level + n, 10)
    elseif k == "giga_folder" then
        stats.giga_level = math.min(stats.giga_level + n, 10)
    elseif k == "custom_down" then
        stats.custom_level = math.max(stats.custom_level - n, 2)
    elseif k == "aura" then
        stats.first_barrier = e.barrier
    elseif k == "b_button" then
        stats.buster_weapon = e.weapon
    elseif k == "charged_shot" then
        stats.charge_shot_weapon = e.weapon
    elseif k == "b_left" then
        stats.back_special_weapon = e.weapon
    elseif k == "full_synchro" then
        stats.full_synchro_start = true
    elseif k == "color" then
        stats.color = n
    elseif k == "panel_trail" then
        stats.panel_trail = n
    elseif k == "bug" then
        counts.add(e.what :: string, n)
    elseif k == "clear_bug" then
        counts.clear(e.what :: string)
    else
        error(string.format("patch card effect %q is no kind of EXE4's", tostring(k)))
    end
end

return cards
