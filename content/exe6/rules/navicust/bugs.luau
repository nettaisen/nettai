--!strict
-- EXE6's bugs a hit inflicts: `sub_80139F6`'s NaviCust bugs (@exelib/navicust/bugs, which EXE5's share: a code below
-- 0x64 names a NaviStats byte, EXE6's own stats' by their names; its own codes from 0xF5), the HP bug's codes and the
-- paralyzing, blinding one any navi takes (`sub_801A6B4`, `sub_801A720`), the HP bug's marker (`sub_801A4A6`), and the
-- NaviCust's hit bug once a hit sequence as damage lands (`sub_8013F1E`). EXE5's intake has none of the second and
-- third (its battle code tests none of the codes 0xF4, 0xF6 and 0xF7).

local bugs = require("@exelib/navicust/bugs")
local sparks = require("../../lib/sparks")
local statuses = require("../status")

-- The rules' state field the hit bug keeps (../init.luau declares it).
type State = {
    -- AIData+0x1C: the hit bug took during this hit sequence (held while the navi's `prevent_anim` is set).
    hit_bug_latched: boolean,
}

local GAME: NaviCustBugTable = {
    -- The rules' `stats` by their bytes: the Beast Out counter (+0x21), the sun (+0x22), what the NaviCust does to
    -- the drops and the encounters (+0x26, +0x28), ChpShufl's and NumbrOpn's flags (+0x60; +0x61, which the custom
    -- screen tests for 1).
    own = {
        [0x21] = { name = "beast_out_counter" },
        [0x22] = { name = "sun", as = "flag" },
        [0x26] = { name = "chip_drops" },
        [0x28] = { name = "encounters" },
        [0x60] = { name = "chip_shuffle", as = "flag" },
        [0x61] = { name = "number_open", as = "one" },
    },
    unheld = { [0x20] = "the version (a side's fact, rules/api's `exe6.version`)" },
    drain_by_flags = false,
    specials = {
        [0xFF] = { kind = "blanks" },
        [0xFE] = bugs.panel_trail(4, 4),
        [0xFA] = bugs.panel_trail(4, 2),
        [0xF9] = bugs.panel_trail(4, 1),
        [0xF5] = bugs.panel_trail(3, 1),
        [0xFB] = { kind = "strip" },
        -- (Effect 0xE's spark, sound 0x8E.)
        [0xF8] = { kind = "uninstall", spark = sparks.uninstall, sound = asset.sound("fade") },
    },
    block = 0x64,
    past_block = "nothing",
    statuses = { hit_blind = statuses.blind_1200, hit_confuse = statuses.confuse_960 },
}

local navicust_bugs = {}

-- The rules' `navi_bug`.
navicust_bugs.navi_bug = bugs.navi_bug(GAME)

-- The rules' `hit_bug`: `sub_801A6B4`, then `sub_801A720`.
function navicust_bugs.hit_bug(side: number, navi: Object)
    local c = navi.collision
    local code = bit32.band(c.inflicted_bugs, 0xFF)
    local s = battle.navi(side)
    if code == 0xF4 or code == 0xF7 then
        -- The HP bug up one (to at most 7); 0xF7 only on an HP with a 4 among its decimal digits, else it is gone.
        if code == 0xF7 and not string.find(tostring(navi.hp), "4", 1, true) then
            c.inflicted_bugs = bit32.band(c.inflicted_bugs, 0xFF00)
        else
            s.hp_drain = math.min(s.hp_drain + 1, 7)
        end
    elseif code == 0xF6 then
        -- Both drains up two, paralysis and blindness (`sub_801A77A`; not a navi whose identity is
        -- `bug_blind_immune`, the Cybeasts', NameIDs 0x173 to 0x17E), and the code gone.
        s.hp_drain = math.min(s.hp_drain + 2, 7)
        s.custom_drain = math.min(s.custom_drain + 2, 7)
        navi:take_status(statuses.paralyze_150)
        local identity = navi.identity
        if not (identity and identity.bug_blind_immune) then
            navi:take_status(statuses.blind_1200)
        end
        c.inflicted_bugs = bit32.band(c.inflicted_bugs, 0xFF00)
    end
end

-- The rules' `bug_mark`: the HP bug's codes.
function navicust_bugs.bug_mark(_side: number, navi: Object): boolean
    local code = bit32.band(navi.collision.inflicted_bugs, 0xFF)
    return code == 0xF4 or code == 0xF7
end

-- The rules' `navi_flinched`: `sub_8013F1E`'s gate (a hit, damage, once a hit sequence), then the hit bug.
function navicust_bugs.navi_flinched(side: number, navi: Object)
    local c = navi.collision
    if c.hit_flags == 0 then
        return
    end
    -- Three unaligned words from CollisionData+0x82, which the ARM's rotation reads as each damage slot paired with
    -- the one before it.
    local function word(lo: number, hi: number): number
        return lo + hi * 0x10000
    end
    local sum = word(c:element_damage(0), c.final_damage)
        + word(c:element_damage(2), c:element_damage(1))
        + word(c:element_damage(4), c:element_damage(3))
    if sum % 0x100000000 == 0 then
        return
    end
    local s = rules.state() :: State
    if navi.prevent_anim == 0 then
        s.hit_bug_latched = false
    elseif s.hit_bug_latched then
        return
    else
        s.hit_bug_latched = true
    end
    bugs.hit_status_bug(GAME, side, navi)
end

return navicust_bugs
