--!strict
-- MoonBld's blade (attack object #0x85, `sub_80D5C48`): it rides on its
-- owner and sweeps the eight panels around it, one a tick for 8 ticks,
-- starting ahead and going round (each hit highlighted, on one panel, no
-- spark, a slash with hit modifier 3 and bug 0x18 with argument 1; a
-- plain blade, which no spawner makes, hit modifier 3 alone), then goes
-- after 10 ticks.
--
-- Its spawner leaves registers in its panel and position (see `spawn`);
-- its first update overwrites both, so its position is declared scratch.

local collision = require("../../rules/collision")
local regions = require("../../lib/regions")

local blade = {}

export type State = {
    -- A plain blade's hits carry no bug (the original's first parameter).
    plain: boolean,
}

local SPRITE = asset.sprite("moon-blade")
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
-- The hits: a slash, hit modifier 3, no status, bug 0x18 with argument 1
-- (`dword_80D5D50`, 0x01180003: its bytes are the hit modifier, the
-- status, the bug and its argument), or hit modifier 3 alone for a plain
-- blade.
local HIT = { is = collision.slash, hits = collision.hits_navis }
local HIT_MOD, BUG, BUG_ARG = 3, 0x18, 1

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
    local plain = (me.state :: State).plain
    battle.hitbox(me, {
        panel_x = x,
        panel_y = y,
        element = me.element,
        region = regions.single,
        -- None.
        hit_effect = 0xFF,
        target = HIT.hits,
        self_type = HIT.is,
        damage = me.damage,
        stamina = me.stamina,
        hit_mod = HIT_MOD,
        bug = if plain then 0 else BUG,
        bug_arg = if plain then 0 else BUG_ARG,
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

blade.kind = define.kind {
    id = "moonbld/blade",
    pool = "attack",
    state = { plain = "bool" },
    -- `sub_80D5C48`.
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

-- `sub_80D5D34`: a blade on `owner` (`plain`: a plain blade's hits). Its
-- panel is what the spawner's registers held (`panel_x`, `panel_y`:
-- MoonBld's Atk+ bonus and the low byte of its phase routine's address),
-- its element 0; its position is the registers too (the routine's
-- address, 0, and whatever was in r3).
function blade.spawn(owner: Object, panel_x: number, panel_y: number, damage: number, plain: boolean): Object?
    local o = battle.spawn(blade.kind, Vec3.zero)
    if o then
        o.panel_x, o.panel_y = panel_x, panel_y
        o.element = 0
        o:set_damage_word(damage)
        o.alliance, o.flip = owner.alliance, owner.flip
        o.related1 = owner
        local s = o.state :: State
        s.plain = plain
    end
    return o
end

return blade
