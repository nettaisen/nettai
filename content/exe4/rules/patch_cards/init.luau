--!strict
-- EXE4's rules/patch_cards (docs/design/exe4-map.md §8): the cards' part of the reload (0x08035130): after the
-- NaviCust's programs and the bugs' first pass, the card in each slot (the setup's `patch_cards`, a save's 0x464C),
-- the last slot first, runs its handler (0x08041E6A; its effects ./cards.luau's), and the maximum HP is the base and
-- the HP the programs and cards add again (0x08042FD0). (The check after it, 0x08042F60, takes a Regular chip whose MB
-- the Regular memory no longer covers off the folder: no card changes either.) A card's bugs add to the NaviCust's
-- counts, or clear one, which the bugs' second pass then reads (../navicust).

local cards = require("@self/cards")

local patch_cards = {}

-- The slots a save has (0x464C to 0x4652; the reload reads all seven).
patch_cards.SLOTS = 7

-- The cards of `side`'s setup by slot.
local function by_slot(side: number): { [number]: PatchCard }
    local out: { [number]: PatchCard } = {}
    for _, c in rules.setup_of(side).patch_cards :: { { card: PatchCard, slot: number } } do
        out[c.slot] = c.card
    end
    return out
end

-- The cards' part of the reload for `side` (../navicust's `run` calls it between the bugs' passes).
function patch_cards.run(side: number, stats: Navi, work: PatchCardWork, counts: PatchCardCounts)
    local slots = by_slot(side)
    for slot = patch_cards.SLOTS - 1, 0, -1 do
        local card = slots[slot]
        if card ~= nil then
            for _, e in card.effects do
                cards.apply(stats, e, work, counts)
            end
            stats.max_hp = bit32.band(stats.max_base_hp + work.hp, 0xFFFF)
        end
    end
end

-- `validate`'s part: one card a slot, the slots the save has.
function patch_cards.validate(side: number, problem: (text: string, field: string?, entry: number?) -> ())
    local seen: { [number]: boolean } = {}
    for i, c in rules.setup_of(side).patch_cards :: { { card: PatchCard, slot: number } } do
        if c.slot >= patch_cards.SLOTS then
            problem(string.format("patch card %d: slot %d (0 to %d)", i, c.slot, patch_cards.SLOTS - 1), "patch_cards", i)
        elseif seen[c.slot] then
            problem(string.format("patch card %d: slot %d holds another", i, c.slot), "patch_cards", i)
        end
        seen[c.slot] = true
    end
end

return patch_cards
