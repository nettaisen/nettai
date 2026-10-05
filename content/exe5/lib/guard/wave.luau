--!strict
-- EXE5's shock wave (attack object #0x16, 0x080C739C; EXE6's WaveArm wave, `sub_80C6B40`, is the same number with other
-- code): a wave that hits one panel at a time, along its row. Each segment stands on its panel (a solid one: else it
-- is gone at once) with its collision on, after its variant's ticks starts the next segment one panel ahead, and goes
-- when its animation ends (moved off the field, out of sight). A variant may mark the panels it passes (crack,
-- break, or turn them to another type); such a wave is thrown (row 0x0A), the others plain attacks (row 4); either
-- hits navis with hit modifier 3 and no spark. The Guard chips' counterattack is variant 0xC.
--
-- (In the operation battle, never set in a netbattle, each segment also marks the panel ahead, 0x080E9FA4's effect
-- object #0x83, which isn't ported.)

local collision = require("../../rules/collision")

local wave = {}

-- A variant (0x080C7360's rows, by the wave's first parameter): its animation, its ticks on a panel, and what it does
-- to the panels it passes.
export type Variant = { anim: number, ticks: number, panel: ("cracked" | "broken" | "poison")? }

export type State = {
    -- Its row (passed on to the next segment).
    row: number,
}

local ROWS: { [number]: Variant } = {
    [0x0] = { anim = 0, ticks = 0x19 },
    [0x1] = { anim = 0, ticks = 0x14 },
    [0x2] = { anim = 1, ticks = 0x10 },
    [0x3] = { anim = 1, ticks = 0x0C },
    [0x4] = { anim = 2, ticks = 0x0A, panel = "cracked" },
    [0x5] = { anim = 2, ticks = 0x08, panel = "poison" },
    [0x6] = { anim = 0, ticks = 0x19, panel = "cracked" },
    [0x7] = { anim = 0, ticks = 0x14, panel = "cracked" },
    [0x8] = { anim = 1, ticks = 0x10, panel = "poison" },
    [0x9] = { anim = 1, ticks = 0x0C, panel = "poison" },
    [0xA] = { anim = 2, ticks = 0x0A, panel = "poison" },
    [0xB] = { anim = 2, ticks = 0x08, panel = "poison" },
    [0xC] = { anim = 0, ticks = 0x0A },
    [0xD] = { anim = 1, ticks = 0x07 },
    [0xE] = { anim = 2, ticks = 0x04 },
}

-- Its sheet (10-04) and sound (0x0A6).
local SPRITE = asset.sprite("shock-wave")
local SOUND = asset.sound("shock-wave")
local SOLID = 0x10
local HIT_MOD = 3
-- Out of sight: 240 pixels to the right.
local GONE_X = 240

-- Its actions.
local SPREADING, FADING = 0, 4

-- Done: off its panels, and freed at the next update.
local function finish(me: Object)
    me.collision:set_region(nil)
    me:set_lifecycle("destroy")
end

-- At its animation's end: out of sight, off its panels.
local function hide(me: Object): boolean
    if not bit32.btest(me.sprite.frame_flags, 0x80) then
        return false
    end
    me.pos = Vec3.new(GONE_X * 0x10000, me.pos.y, me.pos.z)
    me.collision:set_region(nil)
    return true
end

-- 0x080C74C2: after its ticks, the next segment ahead (EXE6's `sub_80C6CE4`); its look's end hides it.
local function spread(me: Object)
    if me.phase == 0 then
        me.phase = 1
    end
    local t = me.timer - 1
    me.timer = int.u16(t)
    if t == 0 then
        local damage = int.u32(me.damage + me.stamina * 0x10000)
        wave.spawn(me, me.panel_x + me.facing, me.panel_y, (me.state :: State).row, damage)
        me.action = FADING
    end
    hide(me)
end

-- 0x080C7512: at its look's end, gone.
local function fade(me: Object)
    if hide(me) then
        me:set_lifecycle("destroy")
    end
end

-- 0x080C746C: the hits against it resolve and spark; gone with the battle or off solid ground; a hit takes it off
-- its panel; its action; then present.
local function tick(me: Object)
    local c = me.collision
    c:remove()
    me:hit_spark()
    if battle.over() or not field.check(me.panel_x, me.panel_y, SOLID, 0) then
        finish(me)
        return
    end
    if c.hit_flags ~= 0 then
        c:set_region(nil)
    end
    local a = me.action
    if a == SPREADING then
        spread(me)
    elseif a == FADING then
        fade(me)
    else
        error(string.format("the shock wave's action %#x reads past 0x080C74B0", a))
    end
    c:present()
    me:highlight_collision_panels()
end

-- EXE6's `sub_80C6CFC`, EXE5's 0x080C7544: what the variant does to its panel.
local function mark(x: number, y: number, v: Variant)
    if v.panel == "cracked" then
        field.crack(x, y)
    elseif v.panel == "broken" then
        field.shatter(x, y)
    elseif v.panel then
        field.set_type(x, y, v.panel)
    end
end

-- 0x080C73C0: on a solid panel, its look and timing; its panel marked; its collision; the sound.
local function init(me: Object)
    if not field.check(me.panel_x, me.panel_y, SOLID, 0) then
        me:free()
        return
    end
    local v = ROWS[(me.state :: State).row]
    local sprite = me.sprite
    sprite:load(SPRITE)
    sprite.shadow = "hidden"
    me.visible = true
    me.anim, me.anim_loaded = v.anim, v.anim
    sprite:set_animation(v.anim)
    me.timer, me.timer2 = v.ticks, 0
    sprite.palette = 0
    sprite:set_flip(bit32.bxor(me.alliance, me.flip))
    mark(me.panel_x, me.panel_y, v)
    me:set_coordinates_from_panel()
    if not me:create_collision() then
        me:free()
        return
    end
    local is = if v.panel then collision.thrown else collision.attack
    me:setup_collision(is, collision.hits_navis, HIT_MOD)
    me.collision:set_hit_effect(nil)
    me.collision:present()
    battle.play_sound(SOUND)
    me.lifecycle = "update"
end

wave.kind = define.kind {
    id = "shock-wave",
    pool = "attack",
    state = { row = "u8" },
    -- 0x080C739C.
    update = function(me: Object)
        local l = me.lifecycle
        if l == "init" then
            init(me)
            if not me.active then
                return
            end
        elseif l == "update" then
            tick(me)
        else
            me:destroy()
            return
        end
        me:update_sprite()
    end,
}

-- EXE6's `sub_80C6CE4`, EXE5's 0x080C752C: a segment of `owner`'s on (x, y), of row `row`, with the damage word, of no
-- element. (Its position is the spawner's registers until its init places it.)
function wave.spawn(owner: Object, x: number, y: number, row: number, damage: number): Object?
    if not ROWS[row] then
        error(string.format("the shock wave's row %#x reads past 0x080C7360", row))
    end
    local o = battle.spawn(wave.kind, Vec3.new(y, 0, 0))
    if o then
        (o.state :: State).row = row
        o.panel_x, o.panel_y = x, y
        o.element = 0
        o:set_damage_word(damage)
        o.alliance, o.flip = owner.alliance, owner.flip
        o.related1 = owner
    end
    return o
end

return wave
