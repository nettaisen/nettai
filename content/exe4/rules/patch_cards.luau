--!strict
-- EXE4's patch cards (改造カード: docs/design/exe4-map.md §8): what each card does, its constructors (the cards'
-- modules, content/exe4/patch_cards, written by tools/exe4/gen_patch_cards.py from the cards' handlers, the table at
-- 0x08041E8C), and the cards' part of the reload (0x08035130): after the NaviCust's programs and the bugs' first pass,
-- the card in each slot (the setup's `patch_cards`, a save's 0x464C), the last slot first, runs its handler
-- (0x08041E6A), and the maximum HP is the base and the HP the programs and cards add again (0x08042FD0). (The check
-- after it, 0x08042F60, takes a Regular chip whose MB the Regular memory no longer covers off the folder: no card
-- changes either.) A card's bugs add to the NaviCust's counts, or clear one, which the bugs' second pass then reads
-- (./navicust). A card is a `PatchCard` (@exelib's type: EXE4's have no MB, a card a slot) of these effects.

local cards = {}

-- The slots a save has (0x464C to 0x4652; the reload reads all seven).
cards.SLOTS = 7

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

-- The panel a step leaves (+0x1B: EXE4's panel type, 1 broken; `effects.panel_trail`).
function cards.panel_trail(panel: number): PatchCardEffect
    return effect { kind = "panel_trail", amount = panel }
end

-- A bug's count raised (0x080476E0), or cleared (0x080476EC): the bug by its name (`what`).
function cards.bug(bug: string, n: number): PatchCardEffect
    return effect { kind = "bug", what = bug, amount = n }
end
function cards.clear_bug(bug: string): PatchCardEffect
    return effect { kind = "clear_bug", what = bug }
end

-- What the cards' part of the reload works on: the HP the programs and cards add (`hp`), and the bugs' counts.
export type Work = { hp: number }
export type Counts = {
    add: (bug: string, n: number) -> (),
    clear: (bug: string) -> (),
}

local function apply(stats: Navi, e: PatchCardEffect, work: Work, counts: Counts)
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

-- The cards of `side`'s setup by slot.
local function by_slot(side: number): { [number]: PatchCard }
    local out: { [number]: PatchCard } = {}
    for _, c in rules.setup_of(side).patch_cards :: { { card: PatchCard, slot: number } } do
        out[c.slot] = c.card
    end
    return out
end

-- The cards' part of the reload for `side` (./navicust's `run` calls it between the bugs' passes).
function cards.run(side: number, stats: Navi, work: Work, counts: Counts)
    local slots = by_slot(side)
    for slot = cards.SLOTS - 1, 0, -1 do
        local card = slots[slot]
        if card ~= nil then
            for _, e in card.effects do
                apply(stats, e, work, counts)
            end
            stats.max_hp = bit32.band(stats.max_base_hp + work.hp, 0xFFFF)
        end
    end
end

-- `validate`'s part: one card a slot, the slots the save has.
function cards.validate(side: number, problem: (text: string, field: string?, entry: number?) -> ())
    local seen: { [number]: boolean } = {}
    for i, c in rules.setup_of(side).patch_cards :: { { card: PatchCard, slot: number } } do
        if c.slot >= cards.SLOTS then
            problem(string.format("patch card %d: slot %d (0 to %d)", i, c.slot, cards.SLOTS - 1), "patch_cards", i)
        elseif seen[c.slot] then
            problem(string.format("patch card %d: slot %d holds another", i, c.slot), "patch_cards", i)
        end
        seen[c.slot] = true
    end
end

return cards
