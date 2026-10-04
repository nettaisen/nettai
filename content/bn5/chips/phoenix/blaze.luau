--!strict
-- Where Phoenix's fireball lands (BN5's effect object #0x88, 0x080EA4FC; BN5's own): a blaze on the panel, its
-- sprite's animation 0 for 6 ticks, 1 for 21, 2 for 3; then it goes.

local blaze = {}

local SPRITE = asset.sprite("heat-flame")

-- 0x080EA554: each phase's animation and its ticks less one.
local PHASES = { [0] = { 0, 5 }, [4] = { 1, 0x14 }, [8] = { 2, 2 } }

-- 0x080EA542, by phase: its animation (none for the first: its init's), counted up from 0; on to the next past its
-- ticks (the last: gone).
local function tick(me: Object)
    local phase = PHASES[me.phase]
    if not phase then
        error(string.format("Phoenix's blaze: phase %#x reads past its table (0x080EA554)", me.phase))
    end
    if me.phase_init == 0 then
        if me.phase ~= 0 then
            me.anim = phase[1]
        end
        me.timer = 0
        me.phase_init = 4
    end
    local t = int.u16(me.timer + 1)
    me.timer = t
    if t <= phase[2] then
        return
    end
    if me.phase == 8 then
        me:set_lifecycle("destroy")
    else
        me.phase, me.phase_init = me.phase + 4, 0
    end
end

-- 0x080EA520: on its panel, on the ground, animation 0, no shadow; then its first update.
local function init(me: Object)
    local sprite = me.sprite
    sprite:load(SPRITE)
    sprite.shadow = "with_sprite"
    me.anim, me.anim_loaded = 0, 0xFF
    me.visible = true
    me:set_coordinates_from_panel()
    local p = me.pos
    me.pos = Vec3.new(p.x, p.y, 0)
    me:set_lifecycle("update")
    tick(me)
end

blaze.kind = define.kind {
    id = "phoenix/blaze",
    pool = "effect",
    -- 0x080EA4FC.
    update = function(me: Object)
        local l = me.lifecycle
        if l == "init" then
            init(me)
        elseif l == "update" then
            tick(me)
        else
            me:free()
        end
        me:update_sprite()
    end,
}

-- 0x080EA5C2: a blaze on (x, y), of `owner`'s side and facing. (Its position is the spawner's registers, which its
-- init overwrites.)
function blaze.spawn(owner: Object, x: number, y: number): Object?
    local o = battle.spawn(blaze.kind, Vec3.zero)
    if o then
        o.panel_x, o.panel_y = x, y
        o.alliance, o.flip = owner.alliance, owner.flip
        o.related1 = owner
    end
    return o
end

return blaze
