--!strict
-- Rush (the original's actor object #0x4B, `sub_80C3218`), the NaviCust
-- support that stops a chip: the opponent used a chip Rush cancels (its
-- record's flag `rush_cancels`), the ruleset turned the use back
-- (`sub_8010740`) and summoned him through the supports' controller
-- (./controller). He appears by his navi (or at his side's middle), barks
-- for a second, digs in, pops up under the opponent's navi and eats the
-- chip (the opponent's hand moves on, unless the chip is the second
-- WhiCapsl, which doesn't come from the hand), with a bite that hits for
-- one tick; a second later he digs away. Then the controller moves on. See
-- docs/engine/chips.md §2.10.

local collision = require("../../rules/collision")
local effects = require("../effects")
local regions = require("../regions")

export type State = {
    -- The chip he eats, as the ruleset's numeric API gives it (the
    -- original's third and fourth parameters).
    eaten: number,
}

local rush = {}

local SPRITE = asset.sprite("rush")
local APPEAR_SOUND = asset.sound("place")
local BITE_SOUND = asset.sound("bite")
-- The status byte his bite carries.
local STATUS = 0x12
-- The chip he leaves in the hand: the second WhiCapsl
-- (chips/whicapsl-invisible). A chip's number until that chip is a
-- definition (its record stays for the dimming service).
local SPARED_CHIP = 0x17E

-- `byte_80C34A8`: what his navi's panel must be for him to appear there,
-- by side (else at his side's middle).
local STAND = {
    [0] = { require = 0x0001_0000, forbid = 0x20 },
    [1] = { require = 0x0001_0020, forbid = 0 },
}

-- `sub_80C3460`: on his navi's panel if it will do, else at (1 or 6, 2);
-- on the ground.
local function place(me: Object)
    local navi = battle.player(me.alliance)
    local cond = STAND[me.alliance]
    local x, y = me.alliance * 5 + 1, 2
    if navi and field.check(navi.panel_x, navi.panel_y, cond.require, cond.forbid) then
        x, y = navi.panel_x, navi.panel_y
    end
    me.panel_x, me.panel_y = x, y
    me:set_coordinates_from_panel()
    me.pos = Vec3.new(me.pos.x, me.pos.y, 0)
end

-- The enemy side's way: +1 for the left side.
local function enemy_direction(me: Object): number
    return 1 - 2 * me.alliance
end

-- Action 0 (`sub_80C32D8`): his animation 0 plays out, then his bark
-- (animation 1) for 60 ticks.
local function appear(me: Object)
    if me.phase == 0 then
        -- sub_80C32F4
        if bit32.band(me.sprite.frame_flags, 0x80) ~= 0 then
            me.anim = 1
            me.timer = 0x3C
            me.phase, me.phase_init = 4, 0
        end
    elseif me.phase == 4 then
        -- sub_80C330E
        me.timer -= 1
        if me.timer == 0 then
            me:set_action(4)
        end
    else
        error(string.format("Rush's phase %#x reads past its table (off_80C32EC)", me.phase))
    end
end

-- Digging in and out.
local function dust(at: Vec3)
    battle.effect(at, effects.puff)
end

-- Action 4 (`sub_80C3322`): dig in, come up under the opponent, eat the
-- chip, dig away.
local function bite(me: Object)
    local p = me.phase
    if p == 0 then
        -- sub_80C3348
        me.visible = false
        dust(me.pos)
        me.timer = 3
        me.phase, me.phase_init = 4, 0
    elseif p == 4 then
        -- sub_80C3368: under the opponent's navi (gone without one).
        me.timer -= 1
        if me.timer ~= 0 then
            return
        end
        local target = battle.player(bit32.bxor(me.alliance, 1))
        if not target then
            me:set_lifecycle("destroy")
            return
        end
        me.panel_x, me.panel_y = target.panel_x, target.panel_y
        me.collision.panel_x, me.collision.panel_y = target.panel_x, target.panel_y
        me:set_coordinates_from_panel()
        -- The whole-pixel halves of Y and Z go up by one.
        local at = me.pos
        me.pos = Vec3.new(at.x, int.i32(at.y + 0x1_0000), int.i32(at.z + 0x1_0000))
        me.timer = 3
        me.phase, me.phase_init = 8, 0
    elseif p == 8 then
        -- sub_80C33AE
        me.timer -= 1
        if me.timer ~= 0 then
            return
        end
        me.visible = true
        me.anim = 2
        me.phase, me.phase_init = 0xC, 0
    elseif p == 0xC then
        -- sub_80C33CA: the bite: the chip goes from the opponent's hand.
        if bit32.band(me.sprite.frame_flags, 0x80) == 0 then
            return
        end
        if (me.state :: State).eaten ~= SPARED_CHIP then
            local target = battle.player(bit32.bxor(me.alliance, 1))
            if not target then
                -- The branch skips the `pop {r5}`: the game goes on with a
                -- null object and returns into Rush's own memory.
                error("Rush lost the opponent's navi before biting (loc_80C33F4 unbalances the stack)")
            end
            battle.advance_hand(target.alliance)
        end
        me.anim = 3
        me.timer = 0x3C
        me.collision:set_region(regions.single)
        me.phase, me.phase_init = 0x10, 0
    elseif p == 0x10 then
        -- sub_80C3408: the bite hits for a tick; he digs away after 60.
        if bit32.band(me.sprite.frame_flags, 0x40) ~= 0 then
            battle.play_sound(BITE_SOUND)
        end
        me.collision:set_region(nil)
        me.timer -= 1
        if me.timer ~= 0 then
            return
        end
        local at = me.pos
        dust(Vec3.new(at.x - enemy_direction(me) * 0x8_0000, at.y, 0x20_0000))
        me:set_lifecycle("destroy")
    else
        error(string.format("Rush's phase %#x reads past its table (off_80C3334)", p))
    end
end

-- `sub_80C32B6`: the action between taking its hits and registering again.
local function tick(me: Object)
    me.collision:remove()
    local a = me.action
    if a == 0 then
        appear(me)
    elseif a == 4 then
        bite(me)
    else
        error(string.format("Rush's action %#x reads past its table (off_80C32D0)", a))
    end
    me.collision:present()
end

-- `sub_80C323C`: his look, his place, his collision (off until he
-- bites); his first update at once. (Out of collision slots, he frees
-- himself.)
local function init(me: Object)
    local sprite = me.sprite
    sprite:load(SPRITE)
    sprite.shadow = "with_sprite"
    me.anim, me.anim_loaded = 0, 0
    sprite:set_animation(0)
    sprite:set_flip(bit32.bxor(me.alliance, me.flip))
    me.visible = true
    place(me)
    battle.play_sound(APPEAR_SOUND)
    me.element = 0
    me.damage, me.stamina = 0, 0
    if not me:create_collision() then
        me:free()
        return
    end
    me:setup_collision(collision.thrown_piercing_break, collision.hits_navis, 0)
    me.collision.status_base = STATUS
    me.collision:set_region(nil)
    me:set_lifecycle("update")
    tick(me)
end

-- `sub_80C3448`: the controller's flag cleared, gone.
local function leave(me: Object)
    me.collision:free()
    local controller = me.related1
    if controller then
        local c = controller.state :: { out: boolean }
        c.out = false
    end
    me:free()
end

-- `sub_80C3218`: by state, then his sprite (a freed slot's too, which
-- does nothing).
local function update(me: Object)
    local l = me.lifecycle
    if l == "init" then
        init(me)
    elseif l == "update" then
        tick(me)
    else
        leave(me)
    end
    me:update_sprite()
end

rush.kind = define.kind {
    id = "rush",
    pool = "actor",
    state = { eaten = "u16" },
    update = update,
}

-- `sub_80C34B8`: Rush, for `controller` (whose `out` flag he sets, and
-- clears when he goes), eating chip `eaten`.
function rush.spawn(controller: Object, eaten: number): Object?
    local o = battle.spawn(rush.kind, Vec3.zero)
    if not o then
        return nil
    end
    local s = o.state :: State
    s.eaten = eaten
    o.alliance = controller.alliance
    o.related1 = controller
    local c = controller.state :: { out: boolean }
    c.out = true
    o.run_while_dimmed = true
    return o
end

return rush
