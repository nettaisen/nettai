--!strict
-- A falling rock (attack object #0x1D, `sub_80C7D78`): GroundCross's
-- A-charge drops them on the enemies (`sub_8012CB2`). It falls from 104
-- pixels, speeding up 0x2000 a tick, and hits its panel once below 30
-- pixels. On a hit, or on landing on a solid panel, it breaks into two
-- chunks (objects/rock-chunk); on a hole it leaves a puff.

local rock_chunk = require("../rock-chunk/rock_chunk")

local falling_rock = {}

local SPRITE = "10-05"

-- Where it starts, and how fast its fall speeds up, 16.16.
local HEIGHT, GRAVITY = 0x68_0000, 0x2000

-- Below this height (whole pixels) its hit region is on.
local HITS_BELOW = 0x1E

-- As a body: collision types 0x15 against 5, hit modifier 1.
local SELF_TYPE, TARGET, HIT_MOD = 0x15, 5, 1

-- The chunks it breaks into (`dword_80C7F00`): X and Z speeds; their
-- priority; the sound.
local CHUNKS = { { vx = 0x8000, vz = 0x28000 }, { vx = -0x8000, vz = 0x30000 } }
local CHUNK_PRIORITY = 2
local BREAK_SOUND = 0xD9

-- The puff over a hole.
local PUFF = 0x14

-- `sub_80C7F20`: a rock for `owner` on (x, y), of element `element`, with
-- the damage word `damage`. (Its spawn position is the spawner's
-- registers; its init places it. The game can also give it a counter of
-- live rocks to keep, ExtraVars; its only spawner gives none.)
function falling_rock.spawn(owner: Object, x: number, y: number, element: number, damage: number): Object?
    local o = battle.spawn_kind("falling-rock", Vec3.new(y, element, 0))
    if o then
        o.panel_x, o.panel_y = x, y
        o.element = element
        o.damage = bit32.band(damage, 0xFFFF)
        o.stamina = bit32.rshift(damage, 16)
        o.alliance, o.flip = owner.alliance, owner.flip
    end
    return o
end

-- `sub_80C7EC8`: two chunks from where it is (a draw each), and the sound.
local function shatter(me: Object)
    local p = me.pos
    for _, c in CHUNKS do
        local param = bit32.band(battle.rng(), 3) + 1
        rock_chunk.spawn(Vec3.new(p.x, p.y, 0), param, CHUNK_PRIORITY, c.vx, c.vz)
    end
    battle.play_sound(BREAK_SOUND)
end

-- `sub_80C7E6C`: fall; its hits start below 30 pixels (a tick without
-- moving); at the ground it breaks, or puffs over a hole.
local function fall(me: Object)
    if me.phase == 0 and bit32.band(bit32.rshift(me.pos.z, 16), 0xFFFF) < HITS_BELOW then
        me.collision.region = 1
        me.phase, me.phase_init = 4, 0
        return
    end
    local v = me.vel
    local vz = int.i32(v.z + GRAVITY)
    me.vel = Vec3.new(v.x, v.y, vz)
    local p = me.pos
    local z = int.i32(p.z - vz)
    me.pos = Vec3.new(p.x, p.y, z)
    if z > 0 then
        return
    end
    if field.solid(me.panel_x, me.panel_y) then
        shatter(me)
    else
        battle.effect(me.pos, PUFF)
    end
    me.collision.region = 0
    me:set_lifecycle("destroy")
end

-- `sub_80C7E24`: take this tick's hits; broken by one, else fall on.
local function tick(me: Object)
    local c = me.collision
    c:remove()
    me:hit_spark()
    if c.hit_flags ~= 0 then
        shatter(me)
        c.region = 0
        me:set_lifecycle("destroy")
        return
    end
    local a = me.action
    if a ~= 0 then
        error(string.format("falling rock action %#x reads past sub_80C7E24's table", a))
    end
    fall(me)
    me:update_sprite()
    c:present()
end

-- `sub_80C7D98`: over its panel, high up, as a body without a hit region
-- yet; gone at once if the battle is over or the panel is off the field.
local function init(me: Object)
    if battle.over() or not field.valid(me.panel_x, me.panel_y) then
        me:free()
        return
    end
    local x, y = field.center(me.panel_x, me.panel_y)
    me.pos = Vec3.new(x, y, HEIGHT)
    me.visible = true
    local sprite = me.sprite
    sprite:load(SPRITE)
    sprite.shadow = "ground"
    me.anim = 0
    sprite:set_animation(0)
    sprite:set_flip(bit32.bxor(me.alliance, me.flip))
    local v = me.vel
    me.vel = Vec3.new(v.x, v.y, 0)
    if not me:create_collision() then
        me:free()
        return
    end
    me:setup_collision(SELF_TYPE, TARGET, HIT_MOD)
    me.collision.hit_effect = 0
    me.collision.region = 0
    me.collision:present()
    me:set_lifecycle("update")
    tick(me)
end

-- `sub_80C7D78`.
function falling_rock.update(me: Object)
    local l = me.lifecycle
    if l == "init" then
        init(me)
    elseif l == "update" then
        tick(me)
    else
        me:destroy()
    end
end

return falling_rock
