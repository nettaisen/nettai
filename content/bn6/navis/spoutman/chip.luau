--!strict
-- SpoutMan's own chip, DripShwr (the original's `sub_80F15CE`, his action
-- table's entry 0xA): he winds up (35 ticks) and, if he can move and the
-- panel three ahead is free solid ground, jumps there and sprays a drip
-- shower (navis/spoutman/drip_shower) around him for 90 ticks before
-- jumping back; else he only recovers. See docs/engine/standard-chips.md,
-- "Action 0x0A".

local drip_shower = require("./drip_shower")
local link = require("../../lib/link_chips")
local regions = require("@common/regions")

export type State = {
    timer: number,
    -- The panel he jumps to (AIAttackVars+0x16, +0x17).
    target_x: number,
    target_y: number,
}

local STATE = { timer = "u16", target_x = "u8", target_y = "u8" }

local WIND_UP, JUMP, LAND, STAY, SPRAY, RISE, BACK = 0x0, 0x4, 0x8, 0xC, 0x10, 0x14, 0x18
local ANIM_WIND_UP, ANIM_READY, ANIM_RISE, ANIM_DOWN = 0x11, 0x12, 4, 3

-- The wind-up: 35 ticks, ready at 30 left.
local WIND_UP_TICKS, READY_AT = 0x23, 0x1E

-- Where he jumps: three panels ahead, solid ground with no body, object,
-- hole or reservation on it (`dword_80F168C`).
local JUMP_PANELS = 3
local JUMP_REQUIRE, JUMP_FORBID = 0x10, 0x0F88_0080

-- The jump takes 2 ticks; recovering without one 2 and 8.
local JUMP_TICKS, STAY_TICKS, STAY_RECOVER_TICKS = 2, 2, 8

-- The shower: 2 turns of a quarter every 10 ticks, not following his
-- action. Its spawner leaves its row, the element and 10 << 16 as its
-- position.
local SHOWER: DripShower = { turns = 2, quarter_ticks = 0xA, follows_action = false }
local SHOWER_Z = 0xA_0000

local RISE_TICKS, BACK_TICKS = 6, 8

-- `sub_80F15FC`: the wind-up; 35 ticks after the entry, ready at 30 left.
local function wind_up(me: Object, s: State)
    if me.step_init == 0 then
        me.step_init = 4
        me.anim = ANIM_WIND_UP
        s.timer = WIND_UP_TICKS
        return
    end
    local t = s.timer - 1
    s.timer = t
    if t == READY_AT then
        me.anim = ANIM_READY
    elseif t <= 0 then
        link.go(me, JUMP)
    end
end

-- `sub_80F162C` (one tick): jump if he can move and the panel three ahead
-- takes him (both panels his), else stay.
local function jump(me: Object, s: State)
    if me:can_move() then
        local x, y = me.panel_x + me.facing * JUMP_PANELS, me.panel_y
        if field.check(x, y, JUMP_REQUIRE, JUMP_FORBID) then
            s.target_x, s.target_y = x, y
            me:reserve_panel(x, y)
            me.future_panel_x, me.future_panel_y = me.panel_x, me.panel_y
            me:reserve_panel(me.panel_x, me.panel_y)
            me.collision:set_region(nil)
            me:set_status("moving", true)
            s.timer = JUMP_TICKS
            link.go(me, LAND)
            return
        end
    end
    s.timer = STAY_TICKS
    link.go(me, STAY)
end

-- `sub_80F1694`: 2 ticks, then on the panel ahead.
local function land(me: Object, s: State)
    if not link.run_out(s) then
        return
    end
    me.panel_x, me.panel_y = s.target_x, s.target_y
    me:set_coordinates_from_panel()
    me:update_collision_panels()
    link.go(me, SPRAY)
end

-- `sub_80F16B4`, without a jump: 2 ticks before its entry (animation 3),
-- then 8, then idle.
local function stay(me: Object, s: State)
    if me.step_init == 0 then
        if link.run_out(s) then
            me.anim = ANIM_DOWN
            s.timer = STAY_RECOVER_TICKS
            me.step_init = 4
        end
    elseif link.run_out(s) then
        me:exit_attack()
    end
end

-- `sub_80F16E0`: the shower on his panel; 90 ticks.
local function spray(me: Object, s: State)
    if me.step_init == 0 then
        me.step_init = 4
        link.add_bonus(me)
        local element = me.attack_element
        drip_shower.spawn(me, me.panel_x, me.panel_y, element, SHOWER, link.damage_word(me), Vec3.new(me.panel_y, element, SHOWER_Z))
        local q = SHOWER.quarter_ticks
        s.timer = q * 4 * SHOWER.turns + q
    elseif link.run_out(s) then
        link.go(me, RISE)
    end
end

-- `sub_80F172C`: up again; 6 ticks.
local function rise(me: Object, s: State)
    if me.step_init == 0 then
        me.step_init = 4
        me.anim = ANIM_RISE
        me:set_status("moving", true)
        s.timer = RISE_TICKS
    elseif link.run_out(s) then
        link.go(me, BACK)
    end
end

-- `sub_80F1756`: back on his panel, both panels released, the move done;
-- 8 ticks, then idle.
local function back(me: Object, s: State)
    if me.step_init == 0 then
        me.step_init = 4
        me.anim = ANIM_DOWN
        me:unreserve_panel(me.panel_x, me.panel_y)
        me.panel_x, me.panel_y = me.future_panel_x, me.future_panel_y
        me:unreserve_panel(me.panel_x, me.panel_y)
        me:set_coordinates_from_panel()
        me:update_collision_panels()
        me.collision:set_region(regions.single)
        me:set_status("moving", false)
        me:set_status("move_complete", true)
        me:set_status("using_action", false)
        s.timer = BACK_TICKS
    elseif link.run_out(s) then
        me:exit_attack()
    end
end

local PHASES: { [number]: (me: Object, s: State) -> () } = {
    [WIND_UP] = wind_up,
    [JUMP] = jump,
    [LAND] = land,
    [STAY] = stay,
    [SPRAY] = spray,
    [RISE] = rise,
    [BACK] = back,
}

return define.chip {
    id = "bn6:dripshwr",
    codes = { "A" },
    element = "aqua",
    family = "aqua",
    class = "special",
    rarity = 0,
    mb = 99,
    damage = { formula = "navi_level", base = 20, per_level = 10 },
    hit_param = 10,
    lockout = 0,
    flags = { "has_damage", "library", "variable_damage" },
    library = { number = 405, index = 0, sort = 7 },
    slot_in_limit = 1,
    -- `sub_80F15CE` (`off_80F15E0`).
    action = define.action {
        id = "bn6:dripshwr/action",
        state = STATE,
        update = function(me: Object, s: State)
            link.run(me, s, PHASES, "DripShwr (sub_80F15CE)")
        end,
    },
}
