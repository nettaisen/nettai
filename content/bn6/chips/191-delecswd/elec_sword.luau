--!strict
-- ElecMan's own chip, DElecSwd (action 0x0A for AI index 2,
-- `sub_80F09B8`): with an electric glow on him (effect object #0x31 look
-- 7, objects/navi-effect) he slides forward at 5 pixels a tick, and at each
-- panel's center stops if the panel ahead holds an obstacle (flags
-- 0x3800000), the one after it lacks flag 0x10000, or an enemy's body is in
-- region 4 ahead; then he slashes the panel ahead (one hit region with a
-- flash), and comes back to his panel. [unverified: no lab scenario
-- reaches it; docs/engine/standard-chips.md]

local navi_effect = require("../../objects/navi-effect/navi_effect")
local panels = require("../../lib/panels")
local link = require("../190-heatpres/state")
type State = LinkChipState

local routine = {}

local SLIDE, SLASH, RETURN, RECOVER = 0x0, 0x4, 0x8, 0xC
local SPEED = 0x5_0000
-- The glow he wears: look 7.
local GLOW = 7
-- What stops him (`sub_80F0BD2`): panel flags 0x3800000 ahead, flag
-- 0x10000 missing two ahead, an enemy's body (`off_80F0C24`, by side) in
-- region 4 ahead.
local BLOCKED, BEYOND = 0x0380_0000, 0x0001_0000
local ENEMY_BODY = { [0] = 0x0400_0000, 0x0800_0000 }
local REACH = 4
-- The slash: region 4, hit spark 3, target 5, self type 7, hit modifier 1,
-- status 0x10; its flash (effect 0x16, palette +3) 16 pixels up.
local SLASH_HIT = { region = 4, hit_effect = 3, target = 5, self_type = 7, hit_mod = 1, status = 0x10 }
local FLASH, FLASH_PALETTE, FLASH_Z = 0x16, 3, 0x10_0000

-- `sub_80F0BD2`: whether to stop and slash here.
local function stop_here(me: Object): boolean
    local x, y = me.panel_x + me.facing, me.panel_y
    if bit32.btest(field.flags(x, y), BLOCKED) then
        return true
    end
    if not bit32.btest(field.flags(x + me.facing, y), BEYOND) then
        return true
    end
    local px = panels.random_in_region(x, y, ENEMY_BODY[me.alliance], 0, REACH, panels.side_direction(me.alliance))
    return px ~= nil
end

-- `sub_800E708`: `c` lies between `a` and `b`, ends included.
local function between(a: number, b: number, c: number): boolean
    if a == c or b == c then
        return true
    end
    if a > b then
        return c >= b and c <= a
    end
    return c > a and c <= b
end

-- `sub_80F09DC`: glow on, then slide until it is time to slash.
local function slide(me: Object, s: State)
    if me.step_init == 0 then
        me.step_init = 4
        link.add_bonus(me)
        me.anim = 0x10
        me.future_panel_x, me.future_panel_y = me.panel_x, me.panel_y
        me:reserve_panel(me.panel_x, me.panel_y)
        me:set_status("moving", true)
        me:set_status("using_action", true)
        me.vel = Vec3.new(me.facing * SPEED, me.vel.y, me.vel.z)
        local cx, cy = field.center(me.panel_x, me.panel_y)
        s.held = navi_effect.spawn(me, Vec3.new(cx, cy, 0), { GLOW, me.alliance })
        if stop_here(me) then
            me:set_panel_from_coordinates()
            link.go(me, SLASH)
        end
        return
    end
    local cx = field.center(me.panel_x, me.panel_y)
    local vx = me.vel.x
    local x = int.i32(me.pos.x + vx)
    me.pos = Vec3.new(x, me.pos.y, me.pos.z)
    -- (The game tests the center against the velocity, not the old X.)
    if cx ~= vx and between(x, vx, cx) and stop_here(me) then
        me:set_panel_from_coordinates()
        link.go(me, SLASH)
        return
    end
    me:set_panel_from_coordinates()
    me:update_collision_panels()
end

-- `sub_80F0B80`, `sub_80F0BA4`: the hit ahead, toward the enemy's side,
-- and the flash on the panel ahead.
local function strike(me: Object)
    local d = link.damage_word(me)
    battle.hitbox(me, {
        panel_x = me.panel_x + panels.side_direction(me.alliance),
        panel_y = me.panel_y,
        element = me.attack_element,
        region = SLASH_HIT.region,
        hit_effect = SLASH_HIT.hit_effect,
        target = SLASH_HIT.target,
        self_type = SLASH_HIT.self_type,
        damage = d,
        stamina = bit32.rshift(d, 16),
        hit_mod = SLASH_HIT.hit_mod,
        status = SLASH_HIT.status,
    })
    local cx, cy = field.center(me.panel_x + me.facing, me.panel_y)
    battle.effect(Vec3.new(cx, cy, FLASH_Z), FLASH, bit32.bxor(me.alliance, me.flip), FLASH_PALETTE)
end

-- `sub_80F0A7A`: animation 0x11, the slash, sound 0xB0; 20 ticks.
local function slash(me: Object, s: State)
    if me.step_init == 0 then
        me.step_init = 4
        me.anim = 0x11
        s.timer = 0x14
        strike(me)
        battle.play_sound(0xB0)
        local glow = s.held
        if glow then
            glow.anim = 0x10
        end
    elseif link.count_down(s) then
        link.go(me, RETURN)
    end
end

-- `sub_80F0AB6`: back on his panel, the move done, glow off; 3 ticks.
local function return_home(me: Object, s: State)
    if me.step_init == 0 then
        me.step_init = 4
        me.anim = 3
        me.panel_x, me.panel_y = me.future_panel_x, me.future_panel_y
        me:unreserve_panel(me.panel_x, me.panel_y)
        me:set_coordinates_from_panel()
        me:update_collision_panels()
        me:set_status("moving", false)
        me:set_status("move_complete", true)
        me:set_status("using_action", false)
        s.timer = 3
        local glow = s.held
        if glow then
            navi_effect.stop(glow)
        end
    elseif link.count_down(s) then
        link.go(me, RECOVER)
    end
end

-- `sub_80F0B0E`: 30 ticks, then idle.
local function recover(me: Object, s: State)
    if me.step_init == 0 then
        me.step_init = 4
        me.anim = 0
        s.timer = 0x1E
    elseif link.count_down(s) then
        me:exit_attack()
    end
end

-- `sub_80F09B8` (`off_80F09CC`).
function routine.update(me: Object, s: State)
    local step = me.step
    if step == SLIDE then
        slide(me, s)
    elseif step == SLASH then
        slash(me, s)
    elseif step == RETURN then
        return_home(me, s)
    elseif step == RECOVER then
        recover(me, s)
    else
        error(string.format("DElecSwd phase %#x reads past off_80F09CC", step))
    end
end

return routine
