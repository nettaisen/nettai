--!strict
-- The bugs a hit inflicts, as EXE5's and EXE6's share them: a hit's bug code (its collision's `bugs`, the low byte)
-- and argument (the high byte) reach the navi it hits (`inflicted_bugs`), and the navi's side's rules say what they
-- do, by these (a game's rules/navicust/bugs hands them its own, `Game`, and its rules' hooks take them):
--
-- * `navi_bug`: a player's navi takes the NaviCust bug (EXE6's `sub_80139F6`, EXE5's 0x0801103E). A code below 0x64
--   names a NaviStats byte, which takes the argument (`BYTES`, by the stat's name; a game's own stats by its `own`),
--   but the drains' (0x18, 0x19) and the custom screen's damage's (0x54), which add to theirs; the codes from 0xF5
--   are the game's special bugs (`special`).
-- * `hit_bug`: any navi takes the HP bug's codes (`sub_801A6B4`: 0xF4, and 0xF7 on an HP with a decimal 4) and the
--   paralyzing, blinding one (`sub_801A720`: 0xF6).
-- * `bug_mark`: the HP bug's codes show its marker (`sub_801A4A6`).
-- * `navi_damaged`: as damage lands, the NaviCust's hit bug (the `hit_status` stat, `sub_8013F1E`) puts its status
--   in place of the hit's, or raises the HP bug.

local bugs = {}

-- A game's part.
export type Game = {
    -- Its own stats (its rules' `stats`) by the NaviStats byte a code names: the stat, and how the game reads the
    -- byte ("flag": any nonzero byte is true; "one": only 1 is; "signed": as `ldrsb` reads it).
    own: { [number]: { name: string, as: ("byte" | "flag" | "one" | "signed")? } },
    -- Bytes a code may name that no stat holds, and what they are (EXE6's version, a side's fact).
    unheld: { [number]: string }?,
    -- A drain's argument goes by its flags (EXE5's 0x0801103E): with bit 4 the level adds its low four bits (to at
    -- most 7), with bit 5 it loses them (to at least 0), else it rises to them (a level that wouldn't rise is left,
    -- the bug spared: nothing reloads). Else the argument adds (EXE6's, to at most 7).
    drain_by_flags: boolean,
    -- The codes from 0xF5 the game's routine takes as bugs of their own, and those it writes as any other code's
    -- byte (EXE5's 0xF5 and 0xF8: no chip gives them, not ported).
    specials: { [number]: Special },
    -- The statuses: code 0xF6's paralysis and blindness; the hit bug's blindness and confusion (levels 1 and 2).
    statuses: { paralyze: Status, blind: Status, hit_blind: Status, hit_confuse: Status },
}

-- A code's own bug: a panel trail (its kind and level); every buster shot a blank (0xFF); MegaMan's body programs
-- gone (0xFB, `sub_8014080`); uninstalled, with a spark and a sound (0xF8, `sub_80140EE`); or a byte past the block
-- that the game writes as any other code's (not ported).
export type Special =
    { kind: "panel_trail", trail: number, level: number }
    | { kind: "blanks" }
    | { kind: "strip" }
    | { kind: "uninstall", spark: Spark, sound: Sound }
    | { kind: "unported" }

function bugs.panel_trail(trail: number, level: number): Special
    return { kind = "panel_trail", trail = trail, level = level }
end

-- (A code's argument written to its byte, `at`.)
type Write = (s: Navi, v: number, at: number) -> ()

local function byte(name: string): Write
    return function(s: Navi, v: number)
        s[name] = v
    end
end

local function flag(name: string): Write
    return function(s: Navi, v: number)
        s[name] = v ~= 0
    end
end

local function one(name: string): Write
    return function(s: Navi, v: number)
        s[name] = v == 1
    end
end

local function signed(name: string): Write
    return function(s: Navi, v: number)
        s[name] = if v >= 0x80 then v - 0x100 else v
    end
end

-- A halfword's low and high byte.
local function low(name: string): Write
    return function(s: Navi, v: number)
        s[name] = bit32.bor(bit32.band(s[name], 0xFF00), v)
    end
end

local function high(name: string): Write
    return function(s: Navi, v: number)
        s[name] = bit32.bor(bit32.band(s[name], 0x00FF), bit32.lshift(v, 8))
    end
end

-- A byte that names content by the original's number (a weapon routine, a shot program, a barrier): the content has
-- no numbers, so a code can only clear it (to `none`, the byte's none).
local function cleared(name: string, none: number, what: string): Write
    return function(s: Navi, v: number, at: number)
        if v ~= none then
            error(string.format("bug code writes %s 0x%x to NaviStats+0x%x: a %s by number is not supported", what, v, at, what))
        end
        s[name] = nil
    end
end

-- A form byte: only the base form (0), the navi's.
local function form(name: string): Write
    return function(s: Navi, v: number, at: number)
        if v ~= 0 then
            error(string.format("bug code writes form 0x%x to NaviStats+0x%x: a form by number is not supported", v, at))
        end
        s[name] = s.base_form
    end
end

local GAUGES: { [number]: "normal" | "fast" | "slow" } = { [0] = "normal", [1] = "fast", [2] = "slow" }

local function gauge(s: Navi, v: number)
    local g: ("normal" | "fast" | "slow")? = GAUGES[v]
    if not g then
        error(string.format("bug code sets the gauge speed to %d, which is not modeled", v))
    end
    s.gauge = g
end

-- The supports: 0xFF none (the support bug), else a bit each (Rush 1, Beat 2, Tango 4).
local function supports(s: Navi, v: number)
    if v == 0xFF then
        s.support_bug = true
        return
    end
    s.support_bug = false
    s.rush = bit32.btest(v, 1)
    s.beat = bit32.btest(v, 2)
    s.tango = bit32.btest(v, 4)
end

local function navi(_s: Navi, v: number, _at: number): ()
    error(string.format("bug code writes navi 0x%x to NaviStats+0x29: a navi by number is not supported", v))
end

-- The NaviStats bytes both games' stats hold, by the code that names them (but the drains' and the custom screen's
-- damage's, 0x18, 0x19 and 0x54, which add).
local BYTES: { [number]: Write } = {
    [0x01] = byte("attack"),
    [0x02] = byte("rapid"),
    [0x03] = byte("charge"),
    [0x04] = cleared("buster_weapon", 0xFF, "weapon routine"),
    [0x05] = cleared("charge_shot_weapon", 0xFF, "weapon routine"),
    [0x06] = cleared("first_barrier", 0, "barrier"),
    [0x07] = cleared("back_special_weapon", 0xFF, "weapon routine"),
    [0x08] = gauge,
    [0x09] = byte("regular_memory"),
    [0x0A] = byte("custom_level"),
    [0x0B] = byte("mega_level"),
    [0x0C] = byte("giga_level"),
    [0x0D] = supports,
    [0x0E] = byte("mood"),
    [0x10] = byte("element"),
    [0x11] = byte("auto_step"),
    [0x12] = byte("panel_trail"),
    [0x13] = byte("panel_trail_level"),
    [0x14] = byte("buster_blanks"),
    [0x15] = byte("buster_charged"),
    [0x16] = byte("hit_status"),
    [0x17] = form("starting_form"),
    [0x1A] = byte("battle_start_bug"),
    [0x1B] = flag("float_shoes"),
    [0x1C] = flag("air_shoes"),
    [0x1D] = flag("undershirt"),
    [0x23] = flag("super_armor"),
    [0x24] = byte("emotion_bug"),
    [0x29] = navi,
    [0x2B] = byte("navi_variant"),
    [0x2C] = form("form"),
    [0x2D] = byte("folder"),
    [0x2E] = byte("folder_1_regular"),
    [0x2F] = byte("folder_2_regular"),
    [0x31] = byte("step_bug"),
    [0x39] = cleared("a_charge_weapon", 0xFF, "weapon routine"),
    [0x3D] = byte("starting_damage"),
    [0x3E] = low("max_base_hp"),
    [0x3F] = high("max_base_hp"),
    [0x40] = low("hp"),
    [0x41] = high("hp"),
    [0x42] = low("max_hp"),
    [0x43] = high("max_hp"),
    [0x44] = cleared("mode9_a_weapon", 0xFF, "weapon routine"),
    [0x48] = low("back_special_damage"),
    [0x49] = high("back_special_damage"),
    [0x4D] = cleared("buster_shot", 0, "shot program"),
    [0x4F] = cleared("charge_shot_kind", 0, "shot program"),
    [0x50] = low("chip_recovery"),
    [0x51] = high("chip_recovery"),
    [0x52] = flag("status_guard"),
    [0x55] = high("custom_damage"),
    [0x56] = byte("folder_1_tag_a"),
    [0x57] = byte("folder_1_tag_b"),
    [0x58] = byte("folder_2_tag_a"),
    [0x59] = byte("folder_2_tag_b"),
    [0x63] = byte("hand_shrink_turn"),
}

-- Code `at`'s argument `v` written to the byte it names.
local function write(game: Game, s: Navi, at: number, v: number)
    local own = game.own[at]
    if own then
        local as = own.as or "byte"
        local w = if as == "flag"
            then flag(own.name)
            elseif as == "one" then one(own.name)
            elseif as == "signed" then signed(own.name)
            else byte(own.name)
        w(s, v, at)
        return
    end
    local unheld = game.unheld and game.unheld[at]
    if unheld then
        error(string.format("bug code writes NaviStats+0x%x, %s, which no stat holds", at, unheld))
    end
    local w = BYTES[at]
    if w == nil then
        error(string.format("bug code writes NaviStats+0x%x, which is not modeled", at))
    end
    w(s, v, at)
end

-- A drain's level after its code's argument.
local function drained(game: Game, level: number, arg: number): number?
    if not game.drain_by_flags then
        return math.min(level + arg, 7)
    end
    local n = bit32.band(arg, 0xF)
    if bit32.btest(arg, 0x10) then
        return math.min(level + n, 7)
    elseif bit32.btest(arg, 0x20) then
        return math.max(level - n, 0)
    elseif n > level then
        return n
    end
    return nil
end

-- The rules' `navi_bug`: "edited" where the stats changed (the abilities and form flags come back), "spared" where
-- nothing may follow (not even the weapons' reload).
local function navi_bug(game: Game, side: number, navi: Object): ("edited" | "spared")?
    local word = navi.collision.inflicted_bugs
    local code, arg = bit32.band(word, 0xFF), bit32.rshift(word, 8)
    local s = battle.navi(side)
    if code == 0 then
        return nil
    elseif code == 0x18 or code == 0x19 then
        local name = if code == 0x18 then "hp_drain" else "custom_drain"
        local level = drained(game, s[name], arg)
        if level == nil then
            return "spared"
        end
        s[name] = level
        return "edited"
    elseif code == 0x54 then
        -- (A byte store of the halfword plus the argument.)
        local d = s.custom_damage
        s.custom_damage = bit32.bor(bit32.band(d, 0xFF00), bit32.band(d + arg, 0xFF))
        return "edited"
    elseif code >= 0x64 then
        local special = game.specials[code]
        if special == nil then
            return nil
        elseif special.kind == "blanks" then
            s.buster_blanks = 4
            return "edited"
        elseif special.kind == "panel_trail" then
            s.panel_trail = special.trail
            s.panel_trail_level = special.level
        elseif special.kind == "strip" then
            navi:strip_body_programs(true)
        elseif special.kind == "uninstall" then
            -- The same but Undershirt, then the form's flags come back (`sub_801469C`); a spark 16 pixels up
            -- (`sub_80E08C4`) and the sound.
            if navi:strip_body_programs(false) then
                navi:refresh_form_flags()
            end
            battle.spark(navi, navi.pos + Vec3.new(0, 0, 0x10_0000), special.spark)
            battle.play_sound(special.sound)
        else
            error(string.format("bug code 0x%x: the game writes it as a byte past NaviStats' (not ported)", code))
        end
        return nil
    end
    write(game, s, code, arg)
    return "edited"
end

-- The rules' `hit_bug`.
local function hit_bug(game: Game, side: number, navi: Object)
    local c = navi.collision
    local code = bit32.band(c.inflicted_bugs, 0xFF)
    local s = battle.navi(side)
    if code == 0xF4 or code == 0xF7 then
        -- (0xF7 takes on an HP with a 4 among its decimal digits, else it is gone.)
        if code == 0xF7 and not string.find(tostring(navi.hp), "4", 1, true) then
            c.inflicted_bugs = bit32.band(c.inflicted_bugs, 0xFF00)
        else
            s.hp_drain = math.min(s.hp_drain + 1, 7)
        end
    elseif code == 0xF6 then
        -- Both drains up two, paralysis and blindness (`sub_801A77A`; not a navi whose identity is
        -- `bug_blind_immune`, the original's Cybeasts', NameIDs 0x173 to 0x17E), and the code gone.
        s.hp_drain = math.min(s.hp_drain + 2, 7)
        s.custom_drain = math.min(s.custom_drain + 2, 7)
        navi:take_status(game.statuses.paralyze)
        local identity = navi.identity
        if not (identity and identity.bug_blind_immune) then
            navi:take_status(game.statuses.blind)
        end
        c.inflicted_bugs = bit32.band(c.inflicted_bugs, 0xFF00)
    end
end

-- The rules' `bug_mark`.
local function bug_mark(_side: number, navi: Object): boolean
    local code = bit32.band(navi.collision.inflicted_bugs, 0xFF)
    return code == 0xF4 or code == 0xF7
end

-- The rules' `navi_damaged`: the hit bug by its level.
local function navi_damaged(game: Game, side: number, navi: Object)
    local s = battle.navi(side)
    local level = s.hit_status
    if level == 1 then
        navi.collision.status_final = game.statuses.hit_blind
    elseif level == 2 then
        navi.collision.status_final = game.statuses.hit_confuse
    elseif level == 3 then
        if s.hp_drain < 7 then
            s.hp_drain += 1
        end
    elseif level ~= 0 then
        error(string.format("NaviCust hit bug %d reads past its table", level))
    end
end

-- A game's rules' hooks for its bugs.
function bugs.hooks(game: Game)
    return {
        navi_bug = function(side: number, navi: Object): ("edited" | "spared")?
            return navi_bug(game, side, navi)
        end,
        hit_bug = function(side: number, navi: Object)
            hit_bug(game, side, navi)
        end,
        bug_mark = bug_mark,
        navi_damaged = function(side: number, navi: Object)
            navi_damaged(game, side, navi)
        end,
    }
end

return bugs
