--!strict
-- EXE5's party battle: EXE5 DS's Party Battle System (docs/design/exe5ds-map.md §4), the GBA's battle with the DS's
-- party rules. A side brings MegaMan and up to two team navis (its setup's `party`), each a member of the side's party
-- (the engine's `battle.party`, `battle.member`) with its own stats block; the player switches among them in battle or
-- on the custom screen, up to a limit (the DS's 3, NaviCust's NavChg+1 and +2 raising it); a team navi at 0 HP falls
-- back to MegaMan (./knockout), and ToadMan, benched, may rescue the navi on the field once a battle
-- (navis/toadman/rescue). A team navi's PartyCust programs (./partycust) add to its own block as the round is set up.
--
-- This module answers the engine's party hooks: who joins (`round_setup`), the bottom screen's panel's refusals
-- (`switch_requested`), the battle mode where a switch request isn't one (`switch_asked`), the rescue
-- (`navi_falling`), and what a setup may state (`validate`). The supports, the Double Attacks and the busy flags they
-- keep (`member_busy`, `navi_switched`) are their own modules'.

local partycust = require("@self/partycust")

local party = {}

-- The side's level in a party battle that states none: the DS's story rank (0x0203C10C: how many of the save's event
-- flags 0x300 to 0x308 are set, in a row), the init exchange's level of the side (0x021B82B0), at its highest. A team
-- navi's base HP and its attacks' damage are read at it (lib/navi_level, the navis' `party`).
party.LEVEL = 9

-- The switches a side may make a round beside its MegaMan's NavChg programs (the DS's fresh stats' +0x17, 0x0206E108;
-- the game's stat `switch_bonus` is what is over it).
party.SWITCHES = 3

-- The battle mode with no switch limit, where a switch request calls a partner instead of switching (the DS's mode
-- 6: its offline mode with one scripted partner).
local UNLIMITED = 6

-- A refused request's sound (0x69): the panel's, on its player's console.
local REFUSED = asset.sound("cant-jack-in")

-- MegaMan's light test's line (0x02004DD8), the chips' (0x08010118).
local LIGHT = 470
-- A team navi's light/dark value: its record's (+0x44), which 0x08010C00 writes 500 and a battle leaves (a build
-- states none: every value a team navi's record holds in Tango's saves, 500 to 608, reads alike).
local PARTNER_KARMA = 500
-- The battle effect that holds MegaMan's value at 500 (0x08010EDC).
local VALUE_HELD = 0x20000

export type Partner = {
    navi: NaviDef?,
    programs: { PartyCustProgram? },
}

export type Setup = {
    navi: NaviDef?,
    party: { Partner },
    karma: number,
    level: number?,
}

-- The party block's fields the supports keep (0x021B85E8 + side·0x24, docs/design/exe5ds-map.md §4.7): ProtoMan's and
-- Colonel's spent flags and counters (+0, +1, +6, +7), GyroMan's gust spawned and its object (+2, +0x1C), NapalmMan's,
-- TomahawkMan's and MegaMan's timers and whether each is set (+4 and +3, +8 and +0xA, +0x10 and +0x12), each member busy
-- while its support acts (+0xC…+0xE), ShadowMan's and SearchMan's spent flags (+0xF, +0x14), the damage KnightMan's
-- guard has yet to take (+0x16) and his guard (+0x20).
export type Supports = {
    protoman_used: boolean,
    protoman_count: number,
    gyroman_spawned: boolean,
    napalmman_set: boolean,
    napalmman_timer: number,
    colonel_used: boolean,
    colonel_count: number,
    tomahawkman_timer: number,
    tomahawkman_set: boolean,
    busy: { boolean },
    shadowman_used: boolean,
    megaman_timer: number,
    megaman_set: boolean,
    searchman_used: boolean,
    knightman_pending: number,
    gyroman_gust: Object?,
    knightman_guard: Object?,
}

type State = { rescued: boolean, dice_bonus: boolean, supports: Supports }

-- Whether the side has partners.
function party.has_partners(side: number): boolean
    return battle.party(side).count > 1
end

-- 0x0200DEE8: the member of `side`'s party whose block is of `navi`, the first; none offline for side 1.
function party.member_of(side: number, navi: NaviDef): number?
    if not battle.link() and side ~= 0 then
        return nil
    end
    local p = battle.party(side)
    for m = 0, p.count - 1 do
        local s = battle.member(side, m)
        if s and s.navi == navi then
            return m
        end
    end
    return nil
end

-- The member of the side's party that changes form (the DS's navi 0, MegaMan: member 0, ./validate's).
local function megaman_member(side: number): number?
    local p = battle.party(side)
    for m = 0, p.count - 1 do
        local s = battle.member(side, m)
        if s and s.navi.forms ~= nil then
            return m
        end
    end
    return nil
end

-- MegaMan's light/dark value: the save's (`karma`), 500 in a battle that holds it.
function party.megaman_karma(side: number): number
    if bit32.btest(battle.effects(), VALUE_HELD) then
        return 500
    end
    return (rules.setup_of(side) :: Setup).karma
end

-- The light/dark value of the navi on the field (exe5_8012882 reads the side's NaviStats +0x44): MegaMan's while he
-- fights, a partner's its own.
function party.karma(side: number): number
    if battle.party(side).active ~= 0 then
        return PARTNER_KARMA
    end
    return party.megaman_karma(side)
end

-- 0x02004DD8: MegaMan's light test. With MegaMan on the field it fails only at his emotion 5 (EXE5's `worn_out`: a mood
-- of 0, out of a soul, unangry); with a team navi on the field, it fails when MegaMan's block's mood is 0 or his value
-- is under the line. A party without him passes.
function party.light(side: number): boolean
    if battle.navi(side).navi.forms ~= nil then
        return battle.emotion(side) ~= "worn_out"
    end
    local m = megaman_member(side)
    if m == nil then
        return true
    end
    if (battle.member(side, m) :: Navi).mood == 0 then
        return false
    end
    return party.megaman_karma(side) >= LIGHT
end

-- 0x02004E9C: whether the member of `navi` may support (and rescue): in the party with HP, and in the unlimited mode or
-- with its own support flag (NaviCust's Spport for MegaMan, PartyCust's P.Spport for a team navi).
function party.may_support(side: number, navi: NaviDef): boolean
    local m = party.member_of(side, navi)
    if m == nil then
        return false
    end
    local s = battle.member(side, m) :: Navi
    if s.hp == 0 then
        return false
    end
    if battle.mode() == UNLIMITED then
        return true
    end
    return s.supports_party
end

-- ARM 0x020364E4: whether `navi` is a member and not the navi on the field.
function party.benched(side: number, navi: NaviDef): boolean
    if battle.navi(side).navi == navi then
        return false
    end
    local p = battle.party(side)
    for m = 0, p.count - 1 do
        local s = battle.member(side, m)
        if s and s.navi == navi then
            return true
        end
    end
    return false
end

-- The side's level for a team navi's party data (none: a side without partners, whose team navi reads its story's).
function party.level(side: number): number?
    if not party.has_partners(side) then
        return nil
    end
    local s = rules.setup_of(side) :: Setup?
    return if s and s.level ~= nil then s.level else party.LEVEL
end

-- The switches the side may make this round: its member 0's (the DS's MegaMan's save record, 0x02036D2C, read each
-- round).
function party.limit(side: number): number
    local s = battle.member(side, 0) :: Navi
    return party.SWITCHES + s.switch_bonus
end

-- What joins the side's party as the round is set up (0x02097BDC, the members' blocks from the init exchange's): each
-- of the setup's partners in its order, its block the navi's fresh stats with its PartyCust programs, its maximum HP the
-- base at the side's level and the programs' (0x02004C84), which it enters at. (A link battle's partners are Navi
-- Select's, 0x02097DD4; a build states them.)
function party.round_setup(side: number)
    local setup = rules.setup_of(side) :: Setup
    if #setup.party == 0 then
        return
    end
    local level = party.LEVEL
    if setup.level ~= nil then
        level = setup.level
    end
    for _, p in setup.party do
        local navi = p.navi
        local data = if navi then (navi :: any).party else nil
        if navi and data then
            local stats = battle.join_party(side, navi)
            local programs: { PartyCustProgram } = {}
            for _, program in p.programs do
                if program then
                    table.insert(programs, program)
                end
            end
            partycust.apply(stats, data.hp[level + 1], programs)
        end
    end
end

-- 0x02036F80: the bottom screen's panel's refusals of a request: outside the unlimited mode, the round's switches used
-- up; a member at 0 HP (its block's). Each plays the refusal on its player's console.
function party.switch_requested(side: number, member: number): boolean
    if battle.mode() ~= UNLIMITED and battle.party(side).switches >= party.limit(side) then
        battle.play_sound_for(side, REFUSED)
        return false
    end
    local s = battle.member(side, member)
    if s == nil or s.hp == 0 then
        battle.play_sound_for(side, REFUSED)
        return false
    end
    return true
end

-- 0x0203C5F6: in the unlimited mode a switch request is a partner's call, not a switch (the Double Attacks', who answer
-- it there).
function party.switch_asked(_side: number, _navi: Object, _member: number): boolean
    return battle.mode() ~= UNLIMITED
end

-- 0x020048E8, at 0 HP (0x02062C5A, after the last stand): ToadMan rescues the navi when the side passes the light test,
-- he is a benched member with HP who may support, and his rescue isn't spent this battle (the party block's +0xB): the
-- navi asks for its revival (navis/toadman/rescue, the roles' `revival`), and isn't deleted meanwhile.
function party.rescue(side: number, navi: Object, toadman: NaviDef): boolean
    if not party.has_partners(side) or (rules.state_of(side) :: State).rescued then
        return false
    end
    if not party.light(side) or party.member_of(side, toadman) == nil or not party.benched(side, toadman) then
        return false
    end
    if not party.may_support(side, toadman) then
        return false
    end
    navi:set_request("revival", true)
    return true
end

-- The rescue is spent (its action's start sets the party block's +0xB).
function party.spend_rescue(side: number)
    local state = rules.state_of(side) :: State
    state.rescued = true
end

-- 0x02098E44, as a hit on the side's navi shows (0x02062C40, applyDamageToPlayer's counterpart, on the navi's own
-- console): MegaMan's hurt voice, while he is the member on the field (the side's NaviStats +0x58, the member's slot,
-- 0). The console draws its RNG1; with 6 or 7 in its low three bits, a voice of the damage's tier (under 70, under 150,
-- more: 0x02132044's rows, three voices each), one drawn again (0x02001100). The voice is left out, as the switch's
-- are; the draws stay.
function party.navi_hit_shown(side: number, _navi: Object)
    if party.has_partners(side) and battle.party(side).active == 0 then
        if bit32.band(battle.console_rng_positive(side), 7) >= 6 then
            battle.console_rng_positive(side)
        end
    end
end

-- 0x02098DA0, as a chip's use goes on (0x020070D6, after the light and dark chip check): MegaMan's chip voice, a chip
-- that deals damage while he is the member on the field, on his own console: one of three voices of the damage's tier
-- (the attack's damage and bonus: to 80, 139, 200, more; sounds 0x27F on), drawn from its RNG1 (0x02001100). The voice is
-- left out; the draw stays.
function party.chip_ready(side: number, _navi: Object, chip: Chip)
    if party.has_partners(side) and battle.party(side).active == 0 then
        if chip.flags ~= nil and table.find(chip.flags, "has_damage") ~= nil then
            battle.console_rng_positive(side)
        end
    end
end

-- The side's supports' fields of its party block.
function party.supports(side: number): Supports
    return (rules.state_of(side) :: State).supports
end

-- 0x02004B80 (the rules' `member_busy`): a member busy while its support acts refuses a switch to it.
function party.member_busy(side: number, member: number): boolean
    return party.supports(side).busy[member + 1]
end

-- 0x02004B90: any member busy.
function party.any_busy(side: number): boolean
    local busy = party.supports(side).busy
    return busy[1] or busy[2] or busy[3]
end

-- 0x02003B98 (the rules' `fight_started`, from the start banner's end, 0x0208DDD2): the turn's supports start over:
-- ProtoMan's and Colonel's, and NapalmMan's, TomahawkMan's and MegaMan's timers to set again.
function party.fight_started(side: number)
    local s = party.supports(side)
    s.protoman_used, s.protoman_count = false, 0
    s.colonel_used, s.colonel_count = false, 0
    s.napalmman_set, s.tomahawkman_set, s.megaman_set = false, false, false
end

-- NumberMan's DiceBons is spent this battle (the party block's +0x13), and spending it.
function party.dice_bonus_spent(side: number): boolean
    return (rules.state_of(side) :: State).dice_bonus
end

function party.spend_dice_bonus(side: number)
    local state = rules.state_of(side) :: State
    state.dice_bonus = true
end

-- The rules' `overview`'s part: each partner's PartyCust gauge as a list of its own (the setup's `party` at the
-- partner's entry: its programs' blocks against the gauge, a program in it any number of times), and for MegaMan the
-- switches a round allows (`party.limit`), the party's limit, and with partners a stat of its own (NaviCust's NavChg+1
-- and +2 add to the 3).
function party.overview(side: number, out: SetupOverview)
    local setup = rules.setup_of(side) :: Setup
    for i, p in setup.party do
        local data = if p.navi then (p.navi :: any).party else nil
        if data then
            local programs: { PartyCustProgram } = {}
            for _, program in p.programs do
                if program then
                    table.insert(programs, program)
                end
            end
            local list: SetupList = { field = "party", entry = i, cost = "cost", used = partycust.cost(programs), limit = data.gauge, once = false }
            table.insert(out.lists, list)
        end
    end
    -- (The limit is MegaMan's, partners or none: the Party tab says it before any is chosen. The stat is a party's.)
    if setup.navi == nil or setup.navi.forms == nil then
        return
    end
    local limit = party.limit(side)
    out.switches = limit
    if not party.has_partners(side) then
        return
    end
    local stat: SetupStat = { name = "switches", value = limit, added = limit - party.SWITCHES }
    local stats = out.stats or {}
    table.insert(stats, stat)
    out.stats = stats
end

-- `validate`'s part: a party is MegaMan's (the navi that changes form, member 0: the DS's); its partners are team navis
-- (a navi with party data), none twice and none the side's own; each partner's programs fit its gauge; the level a
-- party battle reads is 0 to the DS's highest.
function party.validate(side: number, problem: (p: SetupProblem) -> ())
    local setup = rules.setup_of(side) :: Setup
    local own = setup.navi
    if #setup.party == 0 or own == nil then
        return
    end
    if own.forms == nil then
        problem({ kind = "not_taken", field = "party", text = string.format("party: %s brings no party (MegaMan does)", battle.name(own)) })
        return
    end
    local seen: { [NaviDef]: number } = {}
    for i, p in setup.party do
        local navi = p.navi
        local data = if navi then (navi :: any).party else nil
        if navi == nil or data == nil or navi == own then
            local name = if navi then battle.name(navi) else "nothing"
            problem({ kind = "invalid", field = "party", entry = i, text = string.format("party %d: %s is no team navi", i, name) })
        elseif seen[navi] then
            problem({
                kind = "twice",
                field = "party",
                entry = i,
                other = seen[navi],
                text = string.format("party %d: %s is in the party at %d too", i, battle.name(navi), seen[navi]),
            })
        else
            seen[navi] = i
            local programs: { PartyCustProgram } = {}
            for _, program in p.programs do
                if program then
                    table.insert(programs, program)
                end
            end
            local used = partycust.cost(programs)
            if used > data.gauge then
                problem({
                    kind = "budget",
                    field = "party",
                    entry = i,
                    used = used,
                    limit = data.gauge,
                    text = string.format("party %d: %s's programs fill %d blocks of its %d", i, battle.name(navi), used, data.gauge),
                })
            end
        end
    end
    local level = setup.level
    if level ~= nil and level > party.LEVEL then
        problem({
            kind = "range",
            field = "level",
            value = level,
            least = 0,
            most = party.LEVEL,
            text = string.format("level %d: a party battle's level is 0 to %d", level, party.LEVEL),
        })
    end
end

return party
