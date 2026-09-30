--!strict
-- GunDelSol's sun beam (effect object #0x48, `sub_80E5C2C`): the light
-- shining on the target column. Visual only, but it takes an effect slot
-- and a place in the update order. It follows its owner at a fixed offset
-- and ends itself once the owner's slot is cleared. See
-- objects-and-player.md §A.6.

local data = require("../data/pack")
local slot = require("../lib/slot")

export type State = {
    slot: Slot,
    -- Offset from the owner's position, 16.16.
    offset: Vec3,
    -- Ticks shown (it hums every 11).
    ticks: number,
}

local sun_beam = {
    name = "sun_beam",
    pool = "effect",
    index = 0x48,
    state = { slot = slot.TYPE, offset = "vec3", ticks = "u16" },
}

-- `sub_80E5D12`: a beam at `offset` from `owner`, stored in `which`.
function sun_beam.spawn(owner: Object, look: SunBeamLook, offset: Vec3, which: Slot): Object?
    local o = battle.spawn("effect", sun_beam.index, offset, { look.look, look.palette, 0, 0 })
    if not o then
        return nil
    end
    o.related1 = owner
    o.alliance, o.flip = owner.alliance, owner.flip
    local s = o.state :: State
    s.slot = which
    s.offset = offset
    slot.fill(owner, which, o)
    return o
end

-- `sub_80E5D3E`: end the beam (its next update frees it).
function sun_beam.stop(o: Object)
    o:set_lifecycle("destroy")
end

-- `sub_80E5CA0`: hidden in time stop (unless param 3 is 1); otherwise
-- follow the owner, or end once the slot is cleared.
local function follow(me: Object)
    if me:param(3) ~= 1 and battle.time_stop() then
        me.visible = false
        return
    end
    me.visible = true
    local s = me.state :: State
    local owner = me.related1 :: Object
    if not slot.occupied(owner, s.slot) then
        me.visible = false
        me:set_lifecycle("destroy")
        return
    end
    if s.ticks % 11 == 0 then
        battle.play_sound(0xF9)
    end
    s.ticks += 1
    me.pos = owner.pos + s.offset
    me:update_sprite()
end

-- `sub_80E5C4C`.
local function init(me: Object)
    local look, palette = me:param(1), me:param(2)
    local sprite = me.sprite
    local id = data.sun_beam_looks[look]
    sprite:load(id.category, id.index)
    sprite:set_animation(0)
    sprite:step()
    sprite.shadow = "with_sprite"
    sprite:set_flip(bit32.bxor(me.alliance, me.flip))
    sprite.palette = palette
    me.no_sprite_update = false
    me.anim = 0
    me.anim_loaded = 0
    me.visible = true;
    (me.state :: State).ticks = 0
    me:set_lifecycle("update")
    follow(me)
end

function sun_beam.update(me: Object)
    local l = me.lifecycle
    if l == "init" then
        init(me)
    elseif l == "update" then
        follow(me)
    else
        me:free()
    end
end

return sun_beam
