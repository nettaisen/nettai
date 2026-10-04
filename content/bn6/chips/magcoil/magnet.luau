--!strict
-- MagCoil's magnet (attack object #0x95, `sub_80D7DE4`): it stands out
-- from its user's hand (the user's attach point 0x1F) for its ticks (the
-- original's first parameter), and every 15 ticks pulls the enemies
-- standing in the three panels ahead on the rows above and below toward
-- its own row: a one-tick hit with no damage and a pull modifier (0xE1
-- from above, 0xD1 from below).

local collision = require("../../rules/collision")
local regions = require("@exelib/regions")

local magnet = {}

export type State = {
    -- How long it stands (the original's first parameter).
    ticks: number,
}

local SPRITE = asset.sprite("magnet")
local HAND, PANEL = 0x1F, 0x28
-- An enemy of each side stands there (the other side's body bit).
local ENEMY_BODY = { [0] = 0x0400_0000, 0x0800_0000 }
-- The pulls from the row above and the row below (hit modifiers).
local PULL_DOWN, PULL_UP = 0xE1, 0xD1
-- Ticks between pulls, and the tick (second timer value) they come on.
local CYCLE, PULL_AT = 0x0F, 0x0B

-- `sub_80D7EF6`: pull whoever stands in the three panels from (x, y)
-- ahead with modifier `how`.
local function pull(me: Object, x: number, y: number, how: number)
    local dir = me.facing
    for _ = 1, 3 do
        if bit32.btest(field.flags(x, y), ENEMY_BODY[me.alliance]) then
            battle.hitbox(me, {
                panel_x = x,
                panel_y = y,
                element = 0,
                region = regions.single,
                -- None.
                hit_effect = nil,
                target = collision.hits_navis,
                self_type = collision.guard_breaking,
                damage = 0,
                hit_mod = how,
            })
        end
        x += dir
    end
end

-- `sub_80D7E98`: a pull every 15 ticks, the first 4 ticks in.
local function cycle(me: Object)
    if me.phase == 0 then
        me.phase = 1
        me.timer2 = CYCLE
    end
    if me.timer2 == PULL_AT then
        pull(me, me.panel_x, me.panel_y - 1, PULL_DOWN)
        pull(me, me.panel_x, me.panel_y + 1, PULL_UP)
    end
    local t = me.timer2 - 1
    me.timer2 = t
    if t <= 0 then
        me.phase = 0
    end
end

-- `sub_80D7E7A`: pull until its time is up or the battle is over.
local function tick(me: Object)
    if battle.over() then
        me:set_lifecycle("destroy")
        return
    end
    local t = me.timer - 1
    me.timer = t
    if t <= 0 then
        me:set_lifecycle("destroy")
        return
    end
    cycle(me)
end

-- `sub_80D7E08`: at the user's hand, a panel back.
local function init(me: Object)
    local sprite = me.sprite
    sprite:load(SPRITE)
    sprite.shadow = "with_sprite"
    me.visible = true
    me.anim = 0
    me.anim_loaded = 0
    sprite:set_animation(0)
    sprite.palette = 0
    sprite:set_flip(bit32.bxor(me.alliance, me.flip))
    me.timer = (me.state :: State).ticks
    me:set_coordinates_from_panel()
    local user = me.related1 :: Object
    local hx, hz = user:attach_point(HAND)
    local p = me.pos
    me.pos = Vec3.new(p.x - me.facing * PANEL * 0x10000 + hx * 0x10000, p.y, p.z + hz * 0x10000)
    me:set_lifecycle("update")
    tick(me)
end

magnet.kind = define.kind {
    id = "magcoil/magnet",
    pool = "attack",
    state = { ticks = "u8" },
    -- `sub_80D7DE4`.
    update = function(me: Object)
        local l = me.lifecycle
        if l == "init" then
            init(me)
        elseif l == "update" then
            tick(me)
        else
            me:destroy()
            return
        end
        me:update_sprite()
    end,
}

-- `sub_80D7ED4`: a magnet for `user` on (x, y) standing `ticks`, stored in
-- the user's first related slot (the game passes the slot; MagCoil's is
-- that one). Its position is the spawner's registers (y, the element, 0)
-- until its init places it.
function magnet.spawn(user: Object, x: number, y: number, element: number, ticks: number): Object?
    local o = battle.spawn(magnet.kind, Vec3.new(y, element, 0))
    if o then
        o.panel_x, o.panel_y = x, y
        o.element = element
        o.alliance, o.flip = user.alliance, user.flip
        o.related1 = user
        user.related1 = o
        local s = o.state :: State
        s.ticks = ticks
    end
    return o
end

return magnet
