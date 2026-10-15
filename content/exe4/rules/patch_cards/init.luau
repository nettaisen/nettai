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
            for _, e in card.effects :: { PatchCardEffect } do
                local pair = e.pair
                if pair == nil then
                    cards.apply(stats, e, work, counts)
                else
                    local other = slots[pair.slot]
                    if other ~= nil and other.id == pair.card and not cards.holds(stats, pair.effects[1]) then
                        for _, inner in pair.effects do
                            cards.apply(stats, inner, work, counts)
                        end
                    end
                end
            end
            stats.max_hp = bit32.band(stats.max_base_hp + work.hp, 0xFFFF)
        end
    end
end

-- `validate`'s part: one card a slot, the slots the save has.
function patch_cards.validate(side: number, problem: (p: SetupProblem) -> ())
    local seen: { [number]: boolean } = {}
    for i, c in rules.setup_of(side).patch_cards :: { { card: PatchCard, slot: number } } do
        if c.slot >= patch_cards.SLOTS then
            problem({
                kind = "range",
                field = "patch_cards",
                entry = i,
                value = c.slot,
                least = 0,
                most = patch_cards.SLOTS - 1,
                text = string.format("patch card %d: slot %d (0 to %d)", i, c.slot, patch_cards.SLOTS - 1),
            })
        elseif seen[c.slot] then
            problem({
                kind = "twice",
                field = "patch_cards",
                entry = i,
                text = string.format("patch card %d: slot %d holds another", i, c.slot),
            })
        end
        seen[c.slot] = true
    end
end

-- `build_check`'s part: a build's cards each in the slot it belongs in (its definition's `slot`, the printed card's:
-- ./cards' `SLOT_0A` to `SLOT_0F`). The reload holds no card to a slot (the ROMs have no table of them: it runs
-- whatever card each of the save's seven slots holds, so `validate` leaves a card anywhere in them be, as a recording
-- may have it). But a scanned card goes into its own slot, so a save made by play has each card there, and a build
-- is held to that, as Tango's rules hold one: a card in another slot, the seventh among them, is said, with the slot
-- it is in and its own.
function patch_cards.build_check(side: number, problem: (p: SetupProblem) -> ())
    for i, c in rules.setup_of(side).patch_cards :: { { card: SlottedPatchCard, slot: number } } do
        local own = c.card.slot
        if c.slot < patch_cards.SLOTS and c.slot ~= own then
            problem({
                kind = "slot",
                field = "patch_cards",
                entry = i,
                def = c.card,
                value = c.slot,
                own = own,
                text = string.format("patch card %d (%s): in slot %d, and it belongs in slot %d", i, battle.name(c.card), c.slot, own),
            })
        end
    end
end

-- The kinds of effect that change a number a tool shows beside the navi's others, or a bug's count: a card with any
-- other kind of effect gives the navi an ability.
local NUMBERS: { [string]: boolean } = {
    hp = true,
    buster_attack = true,
    custom = true,
    custom_down = true,
    mega_folder = true,
    giga_folder = true,
    bug = true,
    clear_bug = true,
}

-- `overview`'s part, into `out`: each of the side's cards' effects that gives the navi an ability, as its card's
-- effect. (Their bugs are counted with the NaviCust's: ../navicust says them.)
function patch_cards.overview(side: number, out: SetupOverview)
    for _, c in rules.setup_of(side).patch_cards :: { { card: PatchCard, slot: number } } do
        for at, e in c.card.effects :: { PatchCardEffect } do
            if not NUMBERS[e.kind] then
                local source: SetupAbility = { entry = c.card, effect = at }
                table.insert(out.abilities, source)
            end
        end
    end
end

return patch_cards
