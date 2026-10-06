--!strict
-- The rising bubble, as EXE5 and EXE6 share it (EXE6's effect object #0x14,
-- `sub_80E1CF8`; EXE5's #0x14): each game makes its own kind
-- (`rising_bubble.make_kind`: its sprite). A bubble that floats up two
-- pixels a tick while its animation plays once,
-- then goes. Only drawn. Anubis and PoisPhar's statue sends them up over
-- the poisoned area; Guardian's and Geddon's objects and AntiRecv use it
-- too, a thrown BugBomb or BlkBomb that lands in a hole leaves one as a
-- puff of smoke, and the plus chips' sparkle is one (@exelib/instant/plus).

local rising_bubble = {}

export type State = {
    -- Its sprite palette (its Param1).
    palette: number,
}

local ANIM = 1
-- Up two pixels a tick.
local RISE = 0x2_0000

-- `sub_80E1D54`: step the animation and rise; gone once its last frame
-- ends.
local function rise(me: Object)
    me.sprite:step()
    local p = me.pos
    me.pos = Vec3.new(p.x, p.y, p.z + me.vel.z)
    if bit32.btest(me.sprite.frame_flags, 0x80) then
        me:set_lifecycle("destroy")
    end
end

-- `sub_80E1D18`.
local function init(me: Object, SPRITE: SpriteAsset)
    me.visible = true
    local sprite = me.sprite
    sprite:load(SPRITE)
    sprite.shadow = "with_sprite"
    me.anim, me.anim_loaded = ANIM, ANIM
    sprite:set_animation(ANIM)
    sprite.palette = (me.state :: State).palette
    me.vel = Vec3.new(me.vel.x, me.vel.y, RISE)
    me.lifecycle = "update"
    rise(me)
end

-- A game's bubble kind: its key and its sprite (animation 1).
function rising_bubble.make_kind(id: string, sprite: SpriteAsset): Kind
    return define.kind {
        id = id,
        pool = "effect",
        state = { palette = "u8" },
        -- `sub_80E1CF8`.
        update = function(me: Object)
            local l = me.lifecycle
            if l == "init" then
                init(me, sprite)
            elseif l == "update" then
                rise(me)
            else
                -- `sub_80E1D72`: object_freeMemory.
                me:free()
            end
        end,
    }
end

-- `sub_80E1D7A`: a bubble (of `kind`, a game's) at `pos` (the spawner's r1
-- to r3) in sprite palette `palette` (r4, whose low byte is the object's
-- Param1).
function rising_bubble.spawn(kind: Kind, pos: Vec3, palette: number): Object?
    local o = battle.spawn(kind, pos)
    if o then
        (o.state :: State).palette = palette
    end
    return o
end

return rising_bubble
