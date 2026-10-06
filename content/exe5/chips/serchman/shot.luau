--!strict
-- SearchMan's shot (EXE5's attack object #0x57, 0x080D0D78; EXE6 keeps its code, `sub_80CF5C8`, which no EXE6 chip
-- spawns): a one-tick hit on a panel, 16 pixels up; a miss shows a spark (effect 7) near where it struck,
-- flipped up or down at random. The powered last shot is another hit and shows a shot's impact.

local collision = require("../../rules/collision")
local sparks = require("../../lib/sparks")

local shot = {}

export type State = {
    -- The powered shot (its Param1 29, not 25).
    powered: boolean,
}

local MISS = new.effect { sprite = asset.sprite("hit-sparks"), anim = 0, palette = 0 }

-- 0x080D0D98: on its panel, 16 pixels up, its hit there (gone off the field).
local function init(me: Object)
    if not field.valid(me.panel_x, me.panel_y) then
        me:free()
        return
    end
    me:set_coordinates_from_panel()
    local p = me.pos
    me.pos = Vec3.new(p.x, p.y, 0x10_0000)
    if not me:create_collision() then
        me:free()
        return
    end
    local s = me.state :: State
    me:setup_collision(if s.powered then collision.collision_1d else collision.curse, collision.hits_navis, 3)
    me.collision:set_hit_effect(if s.powered then sparks.chip_destruction else sparks.plain)
    me.collision:present()
    me:set_lifecycle("update")
end

-- 0x080D0DDE: its hits resolved; a miss's spark; gone.
local function tick(me: Object)
    local c = me.collision
    c:remove()
    me:hit_spark()
    if c.hit_flags == 0 then
        local flip = bit32.band(battle.rng_positive(), 2)
        battle.effect(battle.jitter(0xF, me.pos), MISS, flip, 0, 0)
    end
    c:set_region(nil)
    c:free()
    me:free()
end

shot.kind = new.kind {
    id = "serchman/shot",
    pool = "attack",
    state = { powered = "bool" },
    -- 0x080D0D78.
    update = function(me: Object)
        local l = me.lifecycle
        if l == "init" then
            init(me)
        elseif l == "update" then
            tick(me)
        else
            me:destroy()
        end
    end,
}

-- 0x080D0E1E: a shot of `searchman` on (x, y) with his damage word.
function shot.spawn(searchman: Object, x: number, y: number, powered: boolean): Object?
    local o = battle.spawn(shot.kind, Vec3.zero)
    if o then
        o.panel_x, o.panel_y = x, y
        o:set_damage_word(bit32.bor(searchman.damage, bit32.lshift(searchman.stamina, 16)))
        o.alliance, o.flip = searchman.alliance, searchman.flip
        o.run_while_dimmed = true
        ;(o.state :: State).powered = powered
    end
    return o
end

return shot
