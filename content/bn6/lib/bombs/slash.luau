--!strict
-- A bomb's lingering hit (the original's attack object #0xA, `sub_80C5F60`),
-- drawn as a sword slash: what a bomb whose variant leaves it
-- (`after = slash.after(ticks)`) leaves on its panel. It hits there every
-- tick until it hits something, the battle ends, or its ticks run out. No
-- BN6 chip throws such a bomb (the original's bomb kind 1, with 60 ticks).

local collision = require("../../rules/collision")
local sparks = require("../sparks")

local slash = {}

export type State = { ticks: number }

local SPRITE = asset.sprite("bn6:sword")
local HIT_MOD = 3

-- `sub_80C5FDE`: resolve last tick's hits; end with the battle, the timer
-- or a hit, else hit again.
local function tick(me: Object)
    me.collision:remove()
    me:hit_spark()
    if not battle.over() then
        local t = me.timer - 1
        me.timer = t
        if t >= 0 and me.collision.hit_flags == 0 then
            me.collision:present()
            return
        end
    end
    me.visible = false
    me.collision:set_region(nil)
    me:set_lifecycle("destroy")
end

-- `sub_80C5F84`.
local function init(me: Object)
    local sprite = me.sprite
    sprite:load(SPRITE)
    sprite.shadow = "with_sprite"
    me.visible = true
    me.anim = 0
    me.anim_loaded = 0xFF
    me.timer = (me.state :: State).ticks
    me:set_coordinates_from_panel()
    local p = me.pos
    me.pos = Vec3.new(p.x, p.y, 0)
    if not me:create_collision() then
        me:free()
        return
    end
    me:setup_collision(collision.attack, collision.hits_navis, HIT_MOD)
    me.collision:set_hit_effect(sparks.of_element(me.element))
    me.collision:present()
    me:set_lifecycle("update")
    tick(me)
end

slash.kind = define.kind {
    id = "bn6:bomb-slash",
    pool = "attack",
    state = { ticks = "u8" },
    -- `sub_80C5F60`.
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

-- `sub_80C6018`: what a landed bomb leaves (a variant's `after`): a
-- lingering hit on its panel for `ticks`, of its element and damage word.
-- (Its position is the spawner's registers, the panel row and element,
-- until its init places it; it keeps no flip.)
function slash.after(ticks: number): (bomb: Object) -> ()
    return function(bomb: Object)
        local x, y = bomb.panel_x, bomb.panel_y
        local o = battle.spawn(slash.kind, Vec3.new(y, bomb.element, 0))
        if o then
            (o.state :: State).ticks = ticks
            o.panel_x, o.panel_y = x, y
            o.element = bomb.element
            o.related1 = bomb
            o.damage = bomb.damage
            o.stamina = bomb.stamina
            o.alliance = bomb.alliance
        end
    end
end

return slash
