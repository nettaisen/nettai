--!strict
-- MoonBld's blade, as EXE5 and EXE6 share it (EXE6's attack object #0x85, `sub_80D5C48`; EXE5's #0x69, 0x080D31C4, the
-- same code but for its sprite): it rides on its owner and sweeps the eight panels around it, one a tick for 8
-- ticks, starting ahead and going round (each hit highlighted, on one panel, no spark, a slash with hit modifier 3
-- and bug 0x18 with argument 1; a plain blade, which no spawner makes, hit modifier 3 alone), then goes after 10
-- ticks.
--
-- Its spawner leaves registers in its panel and position (see `spawn`); its first update after its init overwrites
-- both, and its Z isn't modeled, so its position is declared scratch. Each game makes its kind with
-- `blade.make_kind(id, look)` (EXE6's chips/moonbld/blade, EXE5's chips/moonbld).

local regions = require("../regions")

local blade = {}

export type State = {
    -- A plain blade's hits carry no bug (the original's first parameter).
    plain: boolean,
}

-- The panels it sweeps, in order (`byte_80D5D24`): dx toward the facing side, dy.
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
-- The hits: hit modifier 3, no status, bug 0x18 with argument 1 (`dword_80D5D50`, 0x01180003: its bytes are the hit
-- modifier, the status, the bug and its argument), or hit modifier 3 alone for a plain blade; what it is and hits
-- are its game's (`dword_80D5D4C`, 0x0705FF01: rows 7 and 5).
local HIT_MOD, BUG, BUG_ARG = 3, 0x18, 1

-- `sub_80D5CC8`: on its owner.
local function follow(me: Object)
    me.pos = (me.related1 :: Object).pos
    me:set_panel_from_coordinates()
end

-- `sub_80D5CDC`: hit the `i`th panel round its owner.
local function sweep(me: Object, look: MoonBladeLook, i: number)
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
        hit_effect = nil,
        target = look.hits,
        self_type = look.is,
        damage = me.damage,
        stamina = me.stamina,
        hit_mod = HIT_MOD,
        bug = if plain then 0 else BUG,
        bug_arg = if plain then 0 else BUG_ARG,
    })
end

-- `sub_80D5C9A`: on the owner; a panel a tick for 8 ticks; gone after 10.
local function tick(me: Object, look: MoonBladeLook)
    follow(me)
    if me.phase_init == 0 then
        me.timer = 0
        me.phase_init = 4
    end
    if me.timer < 8 then
        sweep(me, look, me.timer)
    end
    local t = me.timer + 1
    me.timer = t
    if t > 9 then
        me:set_lifecycle("destroy")
    end
end

-- `sub_80D5C6C`.
local function init(me: Object, look: MoonBladeLook)
    local sprite = me.sprite
    sprite:load(look.sprite)
    sprite:set_animation(0)
    sprite.shadow = "with_sprite"
    me.anim = 0
    me.anim_loaded = 0
    sprite:set_flip(bit32.bxor(me.alliance, me.flip))
    me.visible = true
    me:set_lifecycle("update")
end

-- A game's blade kind `id`, of `look`.
function blade.make_kind(id: string, look: MoonBladeLook): Kind
    return new.kind {
        id = id,
        pool = "attack",
        state = { plain = "bool" },
        -- `sub_80D5C48`.
        update = function(me: Object)
            local l = me.lifecycle
            if l == "init" then
                init(me, look)
            elseif l == "update" then
                tick(me, look)
            else
                me:destroy()
                return
            end
            me:update_sprite()
        end,
    }
end

-- `sub_80D5D34`: a blade of `kind` on `owner` (`plain`: a plain blade's hits). Its panel and position are what the
-- spawner's registers held: `routine` is the address (with the Thumb bit) of the MoonBld phase routine that spawns
-- it, which the action's dispatch left in a register. Its panel X is `panel_x` (MoonBld's Atk+ bonus), its panel Y
-- the routine's low byte, its element 0; its X is the routine's address, its Y 0, and its Z whatever the action's
-- caller left (EXE6's BattleState's address in a plain use, other values in a Beast or a Cross), which nothing reads
-- and which stays 0 here. Its init doesn't move it, so on the tick it comes out it stands far off the screen and
-- isn't drawn; its next update puts it on its owner.
function blade.spawn(kind: Kind, owner: Object, panel_x: number, routine: number, damage: number, plain: boolean): Object?
    local o = battle.spawn(kind, Vec3.new(routine, 0, 0))
    if o then
        o.panel_x, o.panel_y = panel_x, bit32.band(routine, 0xFF)
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
