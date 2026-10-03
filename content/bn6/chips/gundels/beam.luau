--!strict
-- GunDelSol's sun beam (effect object #0x48, `sub_80E5C2C`): the light
-- shining on the target column (CrosOver's Django shines it too). Visual
-- only, but it takes an effect slot and a place in the update order. It
-- follows its owner at a fixed offset and ends itself once the owner's
-- slot is cleared. See objects-and-player.md §A.6.
--
-- Its look (`SunBeam`, types.d.luau: the original's first two
-- parameters, a look of `dword_80E5C28` and a palette) is its spawner's.

local slot = require("@common/slot")

local beam = {}

export type State = {
    slot: Slot,
    -- Offset from the owner's position, 16.16.
    offset: Vec3,
    -- Ticks shown (it hums every 11).
    ticks: number,
    -- Its look.
    sprite: SpriteAsset,
    palette: number,
    -- It shows while the battle is dimmed too (the original's third
    -- parameter, 1: CrosOver's).
    while_dimmed: boolean,
}

local HUM = asset.sound("bn6:sun-beam")
local HUM_TICKS = 11

-- The looks (`dword_80E5C28`): GunDelSol's, and GunDelEX's wider one.
beam.sprites = {
    plain = asset.sprite("bn6:sun-beam"),
    wide = asset.sprite("bn6:sun-beam-ex"),
}

-- `sub_80E5D3E`: end the beam (its next update frees it).
function beam.stop(o: Object)
    o:set_lifecycle("destroy")
end

-- `sub_80E5CA0`: hidden while dimmed (unless it shows then too);
-- otherwise follow the owner, or end once the slot is cleared.
local function follow(me: Object)
    local s = me.state :: State
    if not s.while_dimmed and battle.dimmed() then
        me.visible = false
        return
    end
    me.visible = true
    local owner = me.related1 :: Object
    if not slot.occupied(owner, s.slot) then
        me.visible = false
        me:set_lifecycle("destroy")
        return
    end
    if s.ticks % HUM_TICKS == 0 then
        battle.play_sound(HUM)
    end
    s.ticks += 1
    me.pos = owner.pos + s.offset
    me:update_sprite()
end

-- `sub_80E5C4C`.
local function init(me: Object)
    local s = me.state :: State
    local sprite = me.sprite
    sprite:load(s.sprite)
    sprite:set_animation(0)
    sprite:step()
    sprite.shadow = "with_sprite"
    sprite:set_flip(bit32.bxor(me.alliance, me.flip))
    sprite.palette = s.palette
    me.no_sprite_update = false
    me.anim = 0
    me.anim_loaded = 0
    me.visible = true
    s.ticks = 0
    me:set_lifecycle("update")
    follow(me)
end

beam.kind = define.kind {
    id = "bn6:gundels/beam",
    pool = "effect",
    state = {
        slot = slot.TYPE,
        offset = "vec3",
        ticks = "u16",
        sprite = "sprite",
        palette = "u8",
        while_dimmed = "bool",
    },
    -- `sub_80E5C2C`.
    update = function(me: Object)
        local l = me.lifecycle
        if l == "init" then
            init(me)
        elseif l == "update" then
            follow(me)
        else
            me:free()
        end
    end,
}

-- `sub_80E5D12`: a beam of `look` at `offset` from `owner`, stored in
-- `which`; shown while dimmed too when `while_dimmed`. It starts at the
-- offset itself (its spawner's registers) until its init places it.
function beam.spawn(owner: Object, look: SunBeam, offset: Vec3, which: Slot, while_dimmed: boolean?): Object?
    local o = battle.spawn(beam.kind, offset)
    if not o then
        return nil
    end
    o.related1 = owner
    o.alliance, o.flip = owner.alliance, owner.flip
    local s = o.state :: State
    s.slot = which
    s.offset = offset
    s.sprite = look.sprite
    s.palette = look.palette
    s.while_dimmed = while_dimmed == true
    slot.fill(owner, which, o)
    return o
end

return beam
