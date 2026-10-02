--!strict
-- What a Gregar or Falzar summon shows while it arrives (the original's
-- effect object #0x7D, the Japanese ROMs' 0x080EE300 in the Falzar ROM,
-- 0x080EF630 in the Gregar ROM; docs/engine/beast-chips.md): a look of the
-- summon's sprite, five pixels down the field and up from where the
-- summon is, for as long as it arrives. The leading piece of each summon
-- spawns it (chips/gregar/summon, chips/falzar/summon).

local arrival = {}

export type State = {
    -- The summon's sprite and the animation of it (the original picks both
    -- by its Param1 from two tables, 0x080EE38C and 0x080EE394: Gregar's
    -- sprite and animation 8, Falzar's and 0x0C).
    look: SpriteAsset?,
    anim: number,
    -- How long it shows (its Param2).
    ticks: number,
}

-- How far down the field and up from the summon (its Y and Z, 16.16).
local OFFSET = 0x5_0000

-- 0x080EE324: on the summon's position, offset; its look drawn without a
-- shadow, facing its way; its timer the ticks it was spawned with.
local function init(me: Object)
    local s = me.state :: State
    local owner = me.related1 :: Object
    local p = owner.pos
    me.pos = Vec3.new(p.x, p.y + OFFSET, p.z + OFFSET)
    local sprite = me.sprite
    sprite:load(s.look :: SpriteAsset)
    sprite.shadow = "with_sprite"
    me.visible = true
    me.anim, me.anim_loaded = s.anim, s.anim
    sprite:set_animation(s.anim)
    sprite.palette = 0
    sprite:set_flip(bit32.bxor(me.alliance, me.flip))
    me.timer = s.ticks
    me:set_lifecycle("update")
end

-- 0x080EE396: gone when the battle is over or its time runs out.
local function tick(me: Object)
    if battle.over() then
        me:set_lifecycle("destroy")
        return
    end
    local t = me.timer - 1
    me.timer = t
    if t <= 0 then
        me:set_lifecycle("destroy")
    end
end

arrival.kind = define.kind {
    id = "beast-chips/arrival",
    pool = "effect",
    state = { look = "sprite", anim = "u8", ticks = "u8" },
    -- 0x080EE300: by state (its end `object_freeMemory`), then the sprite,
    -- stepping while dimmed.
    update = function(me: Object)
        local l = me.lifecycle
        if l == "init" then
            init(me)
        elseif l == "update" then
            tick(me)
        else
            me:free()
            return
        end
        me:update_sprite_while_dimmed()
    end,
}

-- 0x080EE3AE: the arrival of `summon`, of its side and facing, showing
-- animation `anim` of `look` for `ticks` ticks (the original's Param1
-- picks the look, Param2 is the ticks). (Spawned at whatever the
-- spawner's registers hold; its init places it on the summon.)
function arrival.spawn(summon: Object, look: SpriteAsset, anim: number, ticks: number): Object?
    local o = battle.spawn(arrival.kind, Vec3.zero)
    if not o then
        return nil
    end
    o.related1 = summon
    o.alliance, o.flip = summon.alliance, summon.flip
    local s = o.state :: State
    s.look, s.anim, s.ticks = look, anim, ticks
    return o
end

return arrival
