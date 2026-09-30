--!strict
-- MoonBld's blade (attack object #0x85, `sub_80D5C48`): it rides on its
-- owner and sweeps the eight panels around it, one a tick for 8 ticks,
-- starting ahead and going round (each hit highlighted, region 1, no spark,
-- collision types 5 and 7, hit modifier 3, status 0x18 and bug 1; with a
-- Param1, hit modifier 3 alone), then goes after 10 ticks.
--
-- Its spawner leaves registers in its panel and position (see `spawn`);
-- its first update overwrites both, so its position is declared scratch.

local moon_blade = {}

local SPRITE = "10-3c"
-- The panels it sweeps, in order (`byte_80D5D24`): dx toward the facing
-- side, dy.
local SWEEP = {
    { 1, 0 },
    { 1, 1 },
    { 0, 1 },
    { -1, 1 },
    { -1, 0 },
    { -1, -1 },
    { 0, -1 },
    { 1, -1 },
}
-- The hits: region 1, no spark, collision types 5 (target) and 7; hit
-- modifier 3, status 0x18, bug 1 (`dword_80D5D50`), or hit modifier 3 alone
-- with a Param1.
local HIT = { region = 1, hit_effect = 0xFF, target = 5, self_type = 7 }

-- `sub_80D5D34`: a blade on `owner`. Its panel is what the spawner's
-- registers held (`panel_x`, `panel_y`: MoonBld's Atk+ bonus and the low
-- byte of its phase routine's address), its element 0; its position is
-- the registers too (the routine's address, 0, and whatever was in r3).
function moon_blade.spawn(owner: Object, panel_x: number, panel_y: number, damage: number, params: { number }): Object?
    local o = battle.spawn_kind("moon-blade", Vec3.zero, params)
    if o then
        o.panel_x, o.panel_y = panel_x, panel_y
        o.element = 0
        o.damage = damage
        o.stamina = bit32.rshift(damage, 16)
        o.alliance, o.flip = owner.alliance, owner.flip
        o.related1 = owner
    end
    return o
end

-- `sub_80D5CC8`: on its owner.
local function follow(me: Object)
    me.pos = (me.related1 :: Object).pos
    me:set_panel_from_coordinates()
end

-- `sub_80D5CDC`: hit the `i`th panel round its owner.
local function sweep(me: Object, i: number)
    local d = SWEEP[i + 1]
    if d == nil then
        error(string.format("moon blade sweep %d reads past byte_80D5D24", i))
    end
    local x, y = me.panel_x + d[1] * me.facing, me.panel_y + d[2]
    field.highlight(x, y)
    local plain = me:param(1) ~= 0
    battle.hitbox(me, {
        panel_x = x,
        panel_y = y,
        element = me.element,
        region = HIT.region,
        hit_effect = HIT.hit_effect,
        target = HIT.target,
        self_type = HIT.self_type,
        damage = me.damage,
        stamina = me.stamina,
        hit_mod = 3,
        status = if plain then 0 else 0x18,
        bug = if plain then 0 else 1,
    })
end

-- `sub_80D5C9A`: on the owner; a panel a tick for 8 ticks; gone after 10.
local function tick(me: Object)
    follow(me)
    if me.phase_init == 0 then
        me.timer = 0
        me.phase_init = 4
    end
    if me.timer < 8 then
        sweep(me, me.timer)
    end
    local t = me.timer + 1
    me.timer = t
    if t > 9 then
        me:set_lifecycle("destroy")
    end
end

-- `sub_80D5C6C`.
local function init(me: Object)
    local sprite = me.sprite
    sprite:load(SPRITE)
    sprite:set_animation(0)
    sprite.shadow = "with_sprite"
    me.anim = 0
    me.anim_loaded = 0
    sprite:set_flip(bit32.bxor(me.alliance, me.flip))
    me.visible = true
    me:set_lifecycle("update")
end

-- `sub_80D5C48`.
function moon_blade.update(me: Object)
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
end

return moon_blade
