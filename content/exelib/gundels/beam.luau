--!strict
-- GunDelSol's sun beam (EXE6's effect object #0x48, `sub_80E5C2C`; EXE5's the same number and code, 0x080E5BD0, but
-- for the dimming test below): the light shining on the target column (EXE6's CrosOver's Django shines it too).
-- Visual only, but it takes an effect slot and a place in the update order. It follows its owner at a fixed
-- offset and ends itself once the owner's slot is cleared.
--
-- Its look (sprite and palette, the original's first two parameters, a look of EXE6's `dword_80E5C28`) is its
-- spawner's; its hum is its game's (`beam.make_kind(id, look)`: EXE6's chips/gundels/beam, EXE5's
-- objects/gundels). EXE5's hides it whenever the battle is dimmed (0x080E5C44): EXE6's third parameter, which shows
-- it then too (CrosOver's), EXE5's has not, and its spawns never set it.

local slot = require("../slot")

local beam = {}

-- A beam's look: its sprite and palette.
export type Look = SunBeamLook

export type State = {
    slot: Slot,
    -- Offset from the owner's position, 16.16.
    offset: Vec3,
    -- Ticks shown (it hums every 11).
    ticks: number,
    -- Its look.
    sprite: SpriteAsset,
    palette: number,
    -- It shows while the battle is dimmed too (the original's third parameter, 1: CrosOver's).
    while_dimmed: boolean,
}

local HUM_TICKS = 11

-- `sub_80E5D3E`: end the beam (its next update frees it).
function beam.stop(o: Object)
    o:set_lifecycle("destroy")
end

-- `sub_80E5CA0`: hidden while dimmed (unless it shows then too); otherwise follow the owner, or end once the slot
-- is cleared.
local function follow(me: Object, hum: Sound)
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
        battle.play_sound(hum)
    end
    s.ticks += 1
    me.pos = owner.pos + s.offset
    me:update_sprite()
end

-- `sub_80E5C4C`.
local function init(me: Object, hum: Sound)
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
    follow(me, hum)
end

-- A game's beam kind `id`, humming `hum`.
function beam.make_kind(id: string, hum: Sound): Kind
    return define.kind {
        id = id,
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
                init(me, hum)
            elseif l == "update" then
                follow(me, hum)
            else
                me:free()
            end
        end,
    }
end

-- `sub_80E5D12`: a beam of `kind` and `look` at `offset` from `owner`, stored in `which`; shown while dimmed too
-- when `while_dimmed`. It starts at the offset itself (its spawner's registers) until its init places it.
function beam.spawn(kind: Kind, owner: Object, look: Look, offset: Vec3, which: Slot, while_dimmed: boolean?): Object?
    local o = battle.spawn(kind, offset)
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
