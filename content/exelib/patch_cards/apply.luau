--!strict
-- Patch cards' application as EXE5's and EXE6's share it (docs/engine/patch-cards.md §1.2; EXE5's 0x08138214,
-- exe5-map.md §15.14): the original keeps a block of slots, one per kind of effect, seeded from the stats the NaviCust
-- made (each game's seed). Each card in the list's order runs its effects in the order of their kinds
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

-- Whether effect `e` is one a card of variant `variant` (the installed card's, its setup record's `variant`: a
-- version's id) has: one of no version's, or of its.
function apply.has(e: PatchCardEffect, variant: string?): boolean
    return e.version == nil or e.version == variant
end

-- Card `card`'s effects as its variant `variant` has them, in the card's order: those of no variant's, and those of
-- its (the Bass-Cross MegaMan card's, each version's ROM's own: docs/design/versions.md §2.1).
function apply.effects(card: PatchCard, variant: string?): { PatchCardEffect }
    local out: { PatchCardEffect } = {}
    for _, e in card.effects :: { PatchCardEffect } do
        if apply.has(e, variant) then
            table.insert(out, e)
        end
    end
    return out
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
function apply.check(installed: { InstalledPatchCard }, mb: number, problem: (p: SetupProblem) -> ())
    local total = 0
    for i, installed_card in installed do
        local c = installed_card.card
        for j = 1, i - 1 do
            if installed[j].card == c then
                problem({
                    kind = "twice",
                    field = "patch_cards",
                    entry = i,
                    text = string.format("the patch card %s is installed twice", battle.name(c)),
                })
                break
            end
        end
        total += c.mb or 0
    end
    if total > mb then
        problem({
            kind = "budget",
            field = "patch_cards",
            used = total,
            limit = mb,
            text = string.format("the patch cards are %d MB, past %d", total, mb),
        })
    end
end

-- The kinds of effect that add to the stat they write where the others set it (each game's copy: the HP drain in the
-- fight and the custom screen's, `min(stat + n, 7)`, or 0 for an effect of 0).
local ADDS: { [string]: boolean } = { hp_drain = true, custom_hp_drain = true }

-- Whether a bug effect is its bug's fix: it writes the value that is no bug (a trail or a hit status of none; no
-- blank and no charged shot; any other, 0).
local function clears(e: PatchCardEffect): boolean
    local any = e :: any
    if e.kind == "panel_trail" then
        return any.trail == "none"
    elseif e.kind == "hit_status" then
        return any.status == "none"
    elseif e.kind == "buster_blanks" then
        return any.blanks == 0 and any.charged == 0
    end
    return (e.amount or 0) == 0
end

-- The most an HP drain's level is (each game's copy: `min(stat + n, 7)`).
local DRAIN_MOST = 7

-- Whether an overview lists effect `e` among the navi's abilities and bugs: what the card screen lists under its
-- abilities (its `group`), and of its parameters the body's element, which no number of the overview shows (the
-- others are the stats', EXE5's soul turns among them: rules/souls/custom).
local function listed(e: PatchCardEffect): boolean
    return e.group == "ability" or e.kind == "body"
end

-- A game's `overview`'s part: of the cards `installed`, into `out`, what its card screen lists under their
-- abilities (an effect's `group`) and the body's element (`listed`): the abilities, each as its card's effect; and
-- the bugs. A bug effect (`bug_kinds`, the game's bug slots) that the card marks as a bug is listed as the bug it
-- is, by the same name the NaviCust's would have, with its level or its amount (`card_bug`, the game's: its
-- rules/navicust knows the bugs' levels); any other effect a card marks as a bug (an ordinary one that is a
-- drawback: Super Armor off, a lost shield) as its card's effect, the card's own line; and a bug slot's value that
-- is neither a bug nor its fix (a holy trail) is an ability. Of two cards' effects of one kind the later is listed,
-- as its write is the one that holds. And the list itself: its cards' MB together against the most a list holds
-- (`mb`), a card once. A card's effects are those its variant has (`apply.has`); an effect is
-- named by its place in the card's list.
--
-- The cards' effects are listed only where the cards apply (`applies`: the navi that changes form, as each game's
-- `apply` has it), and their bug effects only where no card's BugStop holds them back (the copy's `bug_stop`). Where
-- they are, the list follows what the copy leaves in the stats. A card's effect of a kind takes the place of the
-- NaviCust's bug that writes the same stat (a bug of `out.bugs` with that `card` kind, which the NaviCust's part
-- listed before): the copy sets the stat to the card's value, so the bug the NaviCust brought is gone, whether the
-- card's effect is the bug again (listed as the card's brings it) or its fix (nothing listed: a fix is no ability).
-- An HP drain adds instead (`ADDS`): one of 0 clears the NaviCust's, and any other raises it, the two one bug at
-- their levels together.
function apply.overview(
    installed: { InstalledPatchCard },
    mb: number,
    bug_kinds: { [string]: boolean },
    applies: boolean,
    card_bug: (e: PatchCardEffect) -> SetupBug,
    out: SetupOverview
)
    local last: { [string]: number } = {}
    local held: { [string]: PatchCardEffect } = {}
    local total = 0
    for i, installed_card in installed do
        local c = installed_card.card
        total += c.mb or 0
        for _, e in apply.effects(c, installed_card.variant) do
            if listed(e) then
                last[e.kind] = i
                held[e.kind] = e
            end
        end
    end
    local list: SetupList = { field = "patch_cards", cost = "mb", used = total, limit = mb, once = true }
    table.insert(out.lists, list)
    if not applies then
        return
    end
    local stopper = held.bug_stopper
    local stopped = stopper ~= nil and stopper.amount == 1
    -- The NaviCust's bugs as the cards leave them, and the drains of theirs a card raised.
    local standing: { SetupBug } = {}
    local raised: { [string]: boolean } = {}
    for _, bug in out.bugs do
        local e: PatchCardEffect? = if bug.card ~= nil and not stopped then held[bug.card] else nil
        if e == nil then
            table.insert(standing, bug)
        elseif ADDS[e.kind] and not clears(e) then
            bug.level = math.min((bug.level or 0) + (e.amount or 0), DRAIN_MOST)
            raised[e.kind] = true
            table.insert(standing, bug)
        end
    end
    table.clear(out.bugs)
    for _, bug in standing do
        table.insert(out.bugs, bug)
    end
    for i, installed_card in installed do
        local c = installed_card.card
        for at, e in c.effects :: { PatchCardEffect } do
            if apply.has(e, installed_card.variant) and listed(e) and last[e.kind] == i and not (stopped and bug_kinds[e.kind]) then
                local line: SetupSource = { entry = c, effect = at }
                if not bug_kinds[e.kind] then
                    if e.bug then
                        local drawback: SetupBug = { entry = c, effect = at }
                        table.insert(out.bugs, drawback)
                    else
                        table.insert(out.abilities, line)
                    end
                elseif clears(e) or raised[e.kind] then
                    -- (A fix, or a drain the NaviCust's bug took above.)
                elseif e.bug then
                    local bug = card_bug(e)
                    if ADDS[e.kind] then
                        bug.level = math.min(bug.level or 0, DRAIN_MOST)
                    end
                    table.insert(out.bugs, bug)
                else
                    -- (A bug slot's value that is no bug: a holy trail, charged shots alone.)
                    table.insert(out.abilities, line)
                end
            end
        end
    end
end

-- The cards of `installed` over the block (`slot`, seeded by the game; `hp` the working HP, EXE6's
-- `sub_8141AEC`, EXE5's 0x08137B00), each with its effects for its variant (`apply.effects`): the working HP after
-- them, and which slots a card wrote.
function apply.run(
    installed: { InstalledPatchCard },
    game: PatchCardsGame,
    hp: number,
    slot: { [string]: any }
): (number, { [string]: boolean })
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
        for _, e in apply.in_order(apply.effects(c.card, c.variant), game) do
            local k, n = e.kind, e.amount or 0
            if k == "hp_add" then
                hp = clamp(hp + n, 1, 9999)
            elseif k == "hp_sub" then
                hp = clamp(hp - n, 1, 9999)
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
    return hp, written
end

return apply
