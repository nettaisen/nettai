--!strict
-- TrnArrw1, TrnArrw2 and TrnArrw3 (the original's action 0x28,
-- `sub_80ECDFC`): the navi draws the bow (an attachment) and sets an arrow
-- (objects/flying-shot) over each panel ahead that lets one through, the
-- nearest leaving last (after 16, 12, 8, 4 and 0 ticks); 24 ticks later
-- it shoots one more from its own panel at once, and the bow's animation
-- steps on. It recovers for 16 ticks, the bow stepping on again when the
-- navi's animation ends. The three differ only in their damage.

local attachment = require("../../objects/attachment/attachment")
local flying_shot = require("../../objects/flying-shot/flying_shot")
local lockon = require("../../rules/lockon")

export type State = {
    -- Ticks of the draw so far, then the recovery's (AIAttackVars+0x10).
    timer: number,
}

local chips = {}

-- One state for the three chips' draws.
chips.STATE = { timer = "u16" }

-- The phases, on the attack's step.
local DRAW, RECOVER = 0x0, 0x4
-- RECOVER's step_init: waiting for the navi's animation to end, then not.
local UNTIL_ANIM_END, AFTER = 1, 2

-- The bow (the original's attachment row 0x17).
local BOW = attachment.look { sprite = asset.sprite("bn6:bow"), palette = 0, attach_point = 0x17 }
-- The navi's attach point the arrows are shot from.
local HAND = 0x17
-- Panels an arrow passes over, by side (`byte_80ECEAC`): flag 0x10000 and
-- none of the other side's bodies.
local PASSES = { [0] = { require = 0x10000, forbid = 0x05800000 }, [1] = { require = 0x10000, forbid = 0x0A800000 } }
local SOUND_TWANG = asset.sound("bn6:twang")

-- `sub_80ECF00`: an arrow over (x, y), leaving after `delay` ticks: at the
-- panel's center a pixel up, at the navi's hand height (its sprite's
-- attach point, `sub_8018810`).
local function arrow(me: Object, x: number, y: number, delay: number)
    local cx, cy = field.center(x, y)
    local _, az = me:attach_point(HAND)
    local damage = me.attack_damage + me.hit_param * 0x10000
    -- (The game passes the attack variables' address as the shot's word;
    -- an arrow doesn't read it.)
    local pos = Vec3.new(cx, int.i32(cy - 0x10000), int.shl(az, 16))
    flying_shot.launch(me, pos, { variant = flying_shot.variants.arrow, wait = delay, damage = damage })
end

-- `sub_80ECE18`: the arrows, the bow; after 24 ticks the last arrow.
local function draw(me: Object, s: State)
    if me.step_init == 0 then
        -- The Atk+ bonus goes into the attack's damage word.
        local word = me.attack_damage + me.hit_param * 0x10000 + me.extra
        me.attack_damage = bit32.band(word, 0xFFFF)
        me.hit_param = bit32.rshift(word, 16)
        local dir = me.facing
        local x, y = me.panel_x, me.panel_y
        local delay = 0x14
        local passes = PASSES[me.alliance]
        while true do
            x += dir
            delay -= 4
            if not field.check(x, y, passes.require, passes.forbid) then
                break
            end
            arrow(me, x, y, bit32.band(delay, 0xFF))
        end
        me.anim = 0x0A
        me:open_counter_window()
        attachment.attach(me, BOW, "overlay")
        me:set_status("using_action", true)
        s.timer = 0
        me.step_init = 4
    end
    local t = s.timer + 1
    s.timer = t
    if t < 0x18 then
        return
    end
    arrow(me, me.panel_x, me.panel_y, 0)
    battle.play_sound(SOUND_TWANG)
    local bow = me.overlay
    if bow then
        bow.anim += 1
    end
    me.step = RECOVER
    me.step_init = 0
end

-- `sub_80ECEBC`: 16 ticks; when the navi's animation ends the bow steps
-- on; then the bow goes and back to idle.
local function recover(me: Object, s: State)
    local init = me.step_init
    if init == UNTIL_ANIM_END then
        if bit32.btest(me.sprite.frame_flags, 0x80) then
            local bow = me.overlay
            if bow then
                bow.anim += 1
            end
            me.step_init = AFTER
        end
    elseif init ~= AFTER then
        s.timer = 0x0F
        me.step_init = UNTIL_ANIM_END
    end
    local t = s.timer - 1
    s.timer = t
    if t < 0 then
        me.related1 = nil
        me.overlay = nil
        me:exit_attack()
    end
end

-- `sub_80ECDFC`.
local function update(me: Object, s: State)
    local step = me.step
    if step == DRAW then
        draw(me, s)
    elseif step == RECOVER then
        recover(me, s)
    else
        error(string.format("TrnArrw phase %#x reads past its phase table", step))
    end
end

-- A chip's draw.
local function shoot(): Action
    return define.action { state = chips.STATE, update = update }
end

chips[1] = define.chip {
    id = "bn6:trnarrw1",
    codes = { "A", "F", "K" },
    element = "aqua", family = "aqua", class = "standard",
    rarity = 0, mb = 30, damage = 30, hit_param = 30, lockout = 0,
    flags = { "has_damage", "standard_library", "library" },
    library = { number = 23, index = 23, sort = 444 },
    slot_in_limit = 3,
    -- The Beast rush, and its lock-on mode (rules/lockon).
    beast = { lockon = lockon.trnarrw1 },
    action = shoot(),
}
chips[2] = define.chip {
    id = "bn6:trnarrw2",
    codes = { "G", "M", "Z" },
    element = "aqua", family = "aqua", class = "standard",
    rarity = 1, mb = 36, damage = 40, hit_param = 30, lockout = 0,
    flags = { "has_damage", "standard_library", "library" },
    library = { number = 24, index = 24, sort = 445 },
    slot_in_limit = 3,
    -- The Beast rush, and its lock-on mode (rules/lockon).
    beast = { lockon = lockon.trnarrw1 },
    action = shoot(),
}
chips[3] = define.chip {
    id = "bn6:trnarrw3",
    codes = { "M", "S", "Y" },
    element = "aqua", family = "aqua", class = "standard",
    rarity = 2, mb = 42, damage = 50, hit_param = 30, lockout = 0,
    flags = { "has_damage", "standard_library", "library" },
    library = { number = 25, index = 25, sort = 446 },
    slot_in_limit = 3,
    -- The Beast rush, and its lock-on mode (rules/lockon).
    beast = { lockon = lockon.trnarrw1 },
    action = shoot(),
}

return chips
