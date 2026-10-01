--!strict
-- A mote of a dust storm (the original's effect object #0x19,
-- `sub_80E2080`): it drifts down and forward across its panel from a
-- random point, over and over, for its storm's ticks, or until the storm's
-- user leaves the action the storm is tied to (untied, it runs while
-- dimmed).

local mote = {}

export type State = {
    -- The action its storm is tied to (nil: none).
    tied: Action?,
}

local SPRITE = asset.sprite("dust-storm-mote")
-- Where each drift starts: 20 pixels back and 128 up, then a random step
-- of 15 along its line; its speed, 16.16.
local BACK, TOP = 0x14_0000, 0x80_0000
local VX, VZ = 0x2_AAAA, -0x8_8888
local DRIFT_TICKS = 0xF

-- `sub_80E20B0`.
local function init(me: Object)
    me:set_coordinates_from_panel()
    local r = battle.rng_positive() % 15
    local back = -me.facing
    local p = me.pos
    me.pos = Vec3.new(p.x + back * BACK - int.mul32(VX * back, r), p.y, TOP + int.mul32(VZ, r))
    me.timer = DRIFT_TICKS - r
    me.vel = Vec3.new(me.facing * VX, me.vel.y, VZ)
    local sprite = me.sprite
    sprite:load(SPRITE)
    sprite.shadow = "with_sprite"
    me.visible = true
    me.anim, me.anim_loaded = 0, 0
    sprite:set_animation(0)
    sprite.palette = 0
    sprite:set_flip(bit32.bxor(me.alliance, me.flip))
    me:set_lifecycle("update")
end

-- `sub_80E213C`.
local function tick(me: Object)
    if battle.over() then
        me:free()
        return
    end
    local tied = (me.state :: State).tied
    if tied ~= nil then
        if (me.related1 :: Object):navi_action() ~= tied then
            me:free()
            return
        end
        if battle.dimmed() then
            return
        end
    end
    local left = me.timer2 - 1
    me.timer2 = left
    if left <= 0 then
        me:free()
        return
    end
    local p = me.pos
    me.pos = Vec3.new(p.x + me.vel.x, p.y, p.z + me.vel.z)
    local t = me.timer - 1
    me.timer = t
    if t > 0 then
        return
    end
    me:set_coordinates_from_panel()
    local q = me.pos
    me.pos = Vec3.new(q.x - me.facing * BACK, q.y, TOP)
    me.timer = DRIFT_TICKS
end

mote.kind = define.kind {
    id = "dust-storm-mote",
    pool = "effect",
    state = { tied = "action" },
    -- `sub_80E2080`.
    update = function(me: Object)
        -- (Read before a free: the sprite steps by it either way.)
        local untied = (me.state :: State).tied == nil
        local l = me.lifecycle
        if l == "init" then
            init(me)
        elseif l == "update" then
            tick(me)
        else
            me:free()
        end
        if untied then
            me:update_sprite_while_dimmed()
        else
            me:update_sprite_while_paused()
        end
    end,
}

-- `sub_80E21AC`: a mote of `storm`'s on (x, y), tied to `user` running
-- `tied`, for `ticks` ticks. (It spawns at the registers' values, which
-- its init replaces.)
function mote.spawn(storm: Object, user: Object?, x: number, y: number, ticks: number, tied: Action?): Object?
    local o = battle.spawn(mote.kind, Vec3.new(y, 0, 0))
    if o then
        o.panel_x, o.panel_y = x, y
        o.related1 = user
        o.alliance, o.flip = storm.alliance, storm.flip
        o.timer2 = ticks;
        (o.state :: State).tied = tied
    end
    return o
end

return mote
