--!strict
-- An immobilizing hit (attack object #0x3F, `sub_80CC044`): on its first
-- update a one-tick hit on its panel that deals nothing but immobilizes
-- (status 0x42), then 20 ticks before it goes. Instant chip effect 9
-- spawns it (lib/instant-chips/09-immobilize-row).

local immobilizer = {}

-- The hit (`dword_80CC0C8`, `dword_80CC0CC`): one panel, no spark, against
-- bodies (5) as type 0x32, with status 0x42.
local REGION, HIT_EFFECT, TARGET, SELF_TYPE = 0x01, 0xFF, 0x05, 0x32
local STATUS = 0x42

local SOUND = 0xFC

-- `sub_80CC0B2` with `sub_801155A`: one on (x, y) for `owner`, no element
-- and no damage. (Its spawn position is the spawner's registers; its init
-- places it before anything reads it.)
function immobilizer.spawn(owner: Object, x: number, y: number): Object?
    local o = battle.spawn_kind("immobilizer", Vec3.new(y, 0, 0))
    if o then
        o.panel_x, o.panel_y = x, y
        o.element = 0
        o.damage, o.stamina = 0, 0
        o.alliance, o.flip = owner.alliance, owner.flip
        o.related1 = owner
    end
    return o
end

-- `sub_80CC090`: 20 ticks, then gone.
local function linger(me: Object)
    if me.phase_init == 0 then
        me.timer = 0x14
        me.phase_init = 4
    end
    local left = me.timer - 1
    me.timer = left
    if left <= 0 then
        me:set_lifecycle("destroy")
    end
    me.sprite:step()
end

-- `sub_80CC064`: on its panel, on the ground; the hit, then the wait.
local function init(me: Object)
    me:set_coordinates_from_panel()
    local p = me.pos
    me.pos = Vec3.new(p.x, p.y, 0)
    battle.hitbox(me, {
        panel_x = me.panel_x,
        panel_y = me.panel_y,
        element = 0,
        z = 0,
        region = REGION,
        hit_effect = HIT_EFFECT,
        target = TARGET,
        self_type = SELF_TYPE,
        damage = 0,
        stamina = 0,
        status = STATUS,
    })
    battle.play_sound(SOUND)
    me:set_lifecycle("update")
    linger(me)
end

-- `sub_80CC044`.
function immobilizer.update(me: Object)
    local l = me.lifecycle
    if l == "init" then
        init(me)
    elseif l == "update" then
        linger(me)
    else
        me:free()
    end
end

return immobilizer
