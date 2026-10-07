--!strict
-- The NaviCust bugs a hit inflicts, as EXE5's and EXE6's share them: a hit's bug code (its collision's `bugs`, the
-- low byte) and argument (the high byte) reach the navi it hits (`inflicted_bugs`), and the navi's side's rules say
-- what they do (a game's rules/navicust/bugs hands these its part, `Game`, and its rules' hooks call them):
--
-- * `navi_bug`: a player's navi takes the NaviCust bug (EXE6's `sub_80139F6`, EXE5's 0x0801103E). The drains' codes
--   (0x18, 0x19) and the custom screen's damage's (0x54) add to their stats, and the game's own bugs (`specials`)
--   are their own; any other code names a NaviStats byte, which takes the argument (`BYTES`, by the stat's name; the
--   game's own stats by its `own`), within the side's block (`block`) and, in EXE5, past it (`past_block`).
-- * `hit_status_bug`: the NaviCust's hit bug (the `hit_status` stat) puts its status in place of the hit's, or
--   raises the HP bug (EXE6's `sub_8013F1E`, EXE5's 0x0801156E: a table by the stat), where the game's gate lets it.

local bugs = {}

export type Game = NaviCustBugTable
export type Special = NaviCustBugSpecial

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
    end
    local special = game.specials[code]
    if special ~= nil then
        if special.kind == "blanks" then
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
        end
        return nil
    end
    if code < game.block then
        write(game, s, code, arg)
        return "edited"
    elseif game.past_block == "nothing" then
        return nil
    end
    -- (Past the side's block: side 0's codes up to twice the block are side 1's bytes; the rest no stat holds.)
    if side == 0 and code < 2 * game.block then
        write(game, battle.navi(1), code - game.block, arg)
    end
    return "edited"
end

-- A game's rules' `navi_bug` for its part.
function bugs.navi_bug(game: Game): (side: number, navi: Object) -> ("edited" | "spared")?
    return function(side: number, navi: Object): ("edited" | "spared")?
        return navi_bug(game, side, navi)
    end
end

-- The NaviCust's hit bug by its level (`hit_status`): its status in place of the hit's, or the HP bug up one (to at
-- most 7). (The table has nothing past level 3: EXE6's `off_8013F70`, EXE5's 0x08011590, which the original would
-- read on into code.)
function bugs.hit_status_bug(game: Game, side: number, navi: Object)
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

return bugs
