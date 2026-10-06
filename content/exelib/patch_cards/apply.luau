--!strict
-- Patch cards' application as EXE5's and EXE6's share it (docs/engine/patch-cards.md §1.2; EXE5's 0x08138214,
-- exe5-map.md §15.14): the original keeps a block of slots, one per kind of effect, seeded from the stats the NaviCust
-- made (each game's seed). Each switched-on card in the list's order runs its effects in the order of their kinds
-- (the game's, the original's effect numbers; not the card's order), each writing its slot (a later card's write
-- wins); then the slots a card wrote are copied into the stats (each game's copy). A card that lists one effect
-- twice applies the first only. (Two effects of one kind with different choices, say two charged shots, apply in
-- the card's order here; the original orders them by its effect numbers. No card has two.)

local apply = {}

function apply.clamp(v: number, lo: number, hi: number): number
    return if v < lo then lo elseif v > hi then hi else v
end

-- A byte read as the original's `ldrsb` reads it.
function apply.signed_byte(v: number): number
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
function apply.in_order(effects: { PatchCardEffect }, game: PatchCardsGame): { PatchCardEffect }
    local seen: { [any]: boolean } = {}
    type Entry = { e: PatchCardEffect, i: number }
    local list: { Entry } = {}
    for i, e in effects do
        local rank = game.rank[e.kind]
        if rank == nil then
            error(string.format("patch card effect %q is no kind of effect", tostring(e.kind)))
        end
        local choice = game.choice[e.kind]
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
        local ra, rb = game.rank[a.e.kind], game.rank[b.e.kind]
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

-- What is wrong with `installed` (a side's setup's `patch_cards`), each said with `problem(text, entry)` (a card's
-- place in the list, from 1): a card installed twice; the cards' MB together past `mb` (the game's list's).
function apply.check(installed: { InstalledCard }, mb: number, problem: (text: string, entry: number?) -> ())
    local total = 0
    for i, c in installed do
        for j = 1, i - 1 do
            if installed[j].card == c.card then
                problem(string.format("the patch card %s is installed twice", battle.name(c.card)), i)
                break
            end
        end
        total += c.card.mb
    end
    if total > mb then
        problem(string.format("the patch cards are %d MB, past %d", total, mb), nil)
    end
end

-- The switched-on cards of `installed` over the block (`slot`, seeded by the game; `hp` the working HP, EXE6's
-- `sub_8141AEC`, EXE5's 0x08137B00): the working HP after them, and which slots a card wrote.
function apply.run(installed: { InstalledCard }, game: PatchCardsGame, hp: number, slot: { [string]: any }): (number, { [string]: boolean })
    local written: { [string]: boolean } = {}
    local clamp, signed_byte = apply.clamp, apply.signed_byte
    local function put(name: string, v: any)
        slot[name] = v
        written[name] = true
    end
    local function stat(name: string, d: number, lo: number, hi: number)
        put(name, bit32.band(clamp(signed_byte(slot[name]) + d, lo, hi), 0xFF))
    end
    for _, c in installed do
        if c.on then
            for _, e in apply.in_order(c.card.effects, game) do
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
                    put("body", game.bodies[e.element :: string] + 4)
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
                elseif k == "soul_time_add" then
                    -- (EXE5's: -2 to 6, 0x08137FE4.)
                    stat("soul_time", n, -2, 6)
                elseif k == "soul_time_sub" then
                    stat("soul_time", -n, -2, 6)
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
                    put(k, game.hit_status[e.status :: string])
                elseif k == "encounter_element" then
                    put(k, e.what)
                elseif k == "hub_style" then
                    put(k, n)
                elseif e.on ~= nil then
                    put(k, e.on)
                else
                    put(k, n)
                end
            end
        end
    end
    return hp, written
end

return apply
