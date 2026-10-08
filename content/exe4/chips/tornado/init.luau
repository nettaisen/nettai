--!strict
-- Tornado and Static (EXE4 chips 0x008 and 0x01B; their records at 0x080197EC + n * 0x2C): action 0x28 (0x080ECF44:
-- EXE4's own, its variants 1 and 2). The navi (animation 13, a counter window) holds up the fan (attachment row 0x0F,
-- Static's charged 0x26: 0c-16 in palette 0 or 1, its animation twice the navi's number); 10 ticks in, the fan's
-- animation one on and the chip's tornadoes (./tornado) two panels ahead; 42 ticks; then the fan's animation one back,
-- 21 ticks, and back to idle without it. Each chip states its own blow.
-- Their records are written by tools/exe4/gen_content.py (bn6battle-verify) from the EXE4 ROMs.

local attachment = require("../objects/attachment")
local navicust = require("../rules/navicust")
local statuses = require("../rules/status")
local tornado = require("@self/tornado")

-- The navi's number: what it holds animates by it (NaviStats +0x23, MegaMan's 0).
local NAVI_NUMBER = 0
local FAN = asset.sprite("hand-fan")

local ANIM = 13
local HOLD_TICKS, BLOW_AT, RECOVER_TICKS = 42, 32, 20
local AHEAD = 2
local TICKS = 0x20

type State = { timer: number }

-- A blow: the tornadoes on (x, y) and around, with the attack's element and damage word.
type Blow = (me: Object, x: number, y: number, element: number, damage: number) -> ()

local function action(id: string, fan: AttachmentLook, blow: Blow): Action
    -- 0x080ECF64: the fan up; 10 ticks in, the tornadoes and the fan's next animation; 42 ticks.
    local function hold(me: Object, s: State)
        if me.step_init == 0 then
            me:set_animation(ANIM)
            me:open_counter_window()
            attachment.attach(me, fan, "related", 2 * NAVI_NUMBER)
            me:set_status("using_action", true)
            me.step_init = 4
            s.timer = HOLD_TICKS
        end
        if s.timer == BLOW_AT then
            local damage = int.u32(me.attack_damage + me.hit_param * 0x10000 + me.extra)
            blow(me, me.panel_x + AHEAD * me.facing, me.panel_y, me.attack_element, damage)
            local f = me.related1
            if f then
                f.anim += 1
            end
        end
        local t = s.timer - 1
        s.timer = int.u16(t)
        if t <= 0 then
            me.step, me.step_init = 4, 0
        end
    end

    -- 0x080ED000: the fan's animation back; 21 ticks, then back to idle without it.
    local function recover(me: Object, s: State)
        if me.step_init == 0 then
            local f = me.related1
            if f then
                f.anim -= 1
            end
            s.timer = RECOVER_TICKS
            me.step_init = 4
        end
        local t = s.timer - 1
        s.timer = int.u16(t)
        if t < 0 then
            me.related1 = nil
            me:exit_attack()
        end
    end

    return new.action {
        id = id,
        state = { timer = "u16" },
        -- 0x080ECF44.
        update = function(me: Object, s: State)
            local step = me.step
            if step == 0 then
                hold(me, s)
            elseif step == 4 then
                recover(me, s)
            else
                error(string.format("a fan's phase %#x reads past its table (0x080ECF5C)", step))
            end
        end,
    }
end

-- Tornado's blow (0x080D3130): one tornado (its parameters: the first 1, palette 0; no status; 0x20 ticks).
local PLAIN = tornado.look { palette = 0, status = nil, ticks = TICKS }
local function single(me: Object, x: number, y: number, element: number, damage: number)
    tornado.spawn(me, x, y, element, damage, PLAIN)
end

-- Static's blow (0x080D3152): static tornadoes (palette 3) spreading by the navi's kinds of NaviCust bug (rules/
-- navicust's `bug_kinds`, 0x08043014; 3 at most), each level more panels (0x080D31AC) and a longer paralysis
-- (0x080D31BC: none, 90, 120, 150 ticks).
local STATIC = {
    tornado.look { palette = 3, status = nil, ticks = TICKS },
    tornado.look { palette = 3, status = statuses.paralyze_90, ticks = TICKS },
    tornado.look { palette = 3, status = statuses.paralyze_120, ticks = TICKS },
    tornado.look { palette = 3, status = statuses.paralyze_150, ticks = TICKS },
}
local SPREAD = {
    { { 0, 0 } },
    { { 0, 0 }, { 1, 0 } },
    { { 0, 0 }, { 1, 0 }, { 0, -1 }, { 0, 1 } },
    { { 0, 0 }, { 1, 0 }, { 0, -1 }, { 0, 1 }, { 1, -1 }, { 1, 1 } },
}
local function spread(me: Object, x: number, y: number, element: number, damage: number)
    local level = math.min(navicust.bug_kinds(me.alliance), 3) + 1
    local look = STATIC[level]
    for _, d in SPREAD[level] do
        tornado.spawn(me, x + d[1] * me.facing, y + d[2], element, damage, look)
    end
end

local tornado_chip: Chip = {
    action = action("tornado/action", attachment.look { sprite = FAN, palette = 0 }, single),
    codes = { "E", "L", "T" },
    element = "null",
    family = "wind",
    class = "standard",
    rarity = 2,
    mb = 22,
    damage = 20,
    hit_param = 4,
    lockout = 0,
    flags = { "has_damage", "standard_library", "library" },
}

local static: Chip = {
    action = action("static/action", attachment.look { sprite = FAN, palette = 1 }, spread),
    codes = { "B", "G", "Z", "*" },
    element = "null",
    family = "null",
    class = "standard",
    rarity = 2,
    mb = 30,
    damage = 20,
    hit_param = 4,
    lockout = 0,
    flags = { "has_damage", "standard_library", "library" },
}

return { tornado = tornado_chip, static = static }
