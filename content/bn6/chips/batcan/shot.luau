--!strict
-- BatCan's shot (the original's attack object #1, `sub_80C50B8`): the
-- projectile's twin without a look. Unseen, it steps a panel every two
-- ticks from where it appears and ends on its first hit, with the bat's
-- spark; what differs between BatCan's shots is its hit modifier.
-- See docs/engine/shot-chips.md §6.1.
--
-- (Its record also gives a word its hits add into the target's collision
-- data (`sub_80C518C`: +0x64, added into +0xA0), which nothing in the game
-- reads; it isn't kept.)

local collision = require("../../rules/collision")
local sparks = require("../../lib/sparks")

local shot = {}

export type State = {
    hit_mod: number,
}

-- Ticks it waits on a panel after the one it enters on (Timer2).
local WAIT = 1

-- Done: off its panels, and freed at the next update.
local function finish(me: Object)
    me.collision:set_region(nil)
    me:set_lifecycle("destroy")
end

-- `sub_80C5116`: the hits against it resolve and spark; it ends with the
-- battle or on a hit, else it waits out its panel and steps forward.
local function tick(me: Object)
    local c = me.collision
    c:remove()
    me:hit_spark()
    if battle.over() or c.hit_flags ~= 0 then
        finish(me)
    else
        if me.phase_init == 0 then
            me.timer = me.timer2
            me.phase_init = 4
        end
        local left = me.timer - 1
        me.timer = left
        if left < 0 then
            me.panel_x = me.panel_x + me.facing
            if not field.valid(me.panel_x, me.panel_y) then
                finish(me)
            else
                me:set_coordinates_from_panel()
                me:update_collision_panels()
                me.phase_init = 0
            end
        end
    end
    c:present()
end

-- `sub_80C50DC`: on its panel, an attack of its side with its hit
-- modifier and the bat's spark.
local function init(me: Object)
    me:set_coordinates_from_panel()
    me.timer2 = WAIT
    if not me:create_collision() then
        me:free()
        return
    end
    me:setup_collision(collision.attack, collision.hits_navis, (me.state :: State).hit_mod)
    me.collision:set_hit_effect(sparks.bat)
    me.collision:present()
    me:set_lifecycle("update")
    tick(me)
end

shot.kind = define.kind {
    id = "bn6:batcan/shot",
    pool = "attack",
    state = { hit_mod = "u8" },
    -- `sub_80C50B8`.
    update = function(me: Object)
        local l = me.lifecycle
        if l == "init" then
            init(me)
            if not me.active then
                return
            end
        elseif l == "update" then
            tick(me)
        else
            me:destroy()
            return
        end
        me:update_sprite()
    end,
}

-- `sub_80C5176`: a shot from `owner` on panel (x, y) at height `z`
-- (16.16), with `element`, the damage word and the hit modifier; nil if
-- the attack pool is full. It takes the owner's side and flip, runs later
-- this tick, right after its spawner, and keeps its Z.
function shot.fire(owner: Object, x: number, y: number, z: number, element: number, damage: number, hit_mod: number): Object?
    local o = battle.spawn(shot.kind, Vec3.new(0, 0, z))
    if o then
        (o.state :: State).hit_mod = hit_mod
        o.panel_x, o.panel_y = x, y
        o.element = element
        o:set_damage_word(damage)
        o.alliance, o.flip = owner.alliance, owner.flip
        o.related1 = owner
    end
    return o
end

return shot
