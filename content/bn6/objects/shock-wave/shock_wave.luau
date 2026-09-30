--!strict
-- A shock wave (attack object #0x16, `sub_80C6B40`): WaveArm's and
-- PwrWave's, and the wave viruses send. It stands on its panel for its
-- variant's ticks (its first parameter picks a row of
-- data.objects.shock_waves), sends the next wave on, and goes when its
-- animation ends (WaveArm's and PwrWave's, variants 0xC and up: 4 ticks
-- after sending the next). It needs a solid panel under it, and stops
-- hitting once it has hit something. WaveArm's (variants 0xC to 0xE)
-- steer: toward the row of an enemy just ahead and to the side, once, and
-- keep that row change.

local panels = require("../../lib/panels")

local shock_wave = {}

local SOLID = 0x10
-- The regions it looks for enemies in (`byte_80C6DB8`), by facing (right,
-- left) and panel column.
local AHEAD = {
    [0] = { [0] = 0x08, 0x20, 0x07, 0x06, 0x02, 0x01, 0x01, 0x01 },
    [1] = { [0] = 0x01, 0x01, 0x01, 0x02, 0x06, 0x07, 0x20, 0x08 },
}
-- Up and down a row (`dword_80C6DC8`), by its second parameter (1 up, 2
-- down; the word's other bytes are 0), and where it looks for an enemy
-- before turning.
local ROW_TURN = { -1, 1, 0, 0 }
local UP_REGION, DOWN_REGION = 0x16, 0x17
-- Where it goes when it is done: x 240 pixels, off the field.
local OFF_FIELD = 0xF0

-- `sub_80C6CE4`: a wave on (x, y) from `from` (a navi or the wave before),
-- with the damage word `damage` and parameters `params`. Its element is
-- always null. Its position is the spawner's registers (y, whatever, 0)
-- until its init places it.
function shock_wave.spawn(from: Object, x: number, y: number, params: { number }, damage: number): Object?
    local o = battle.spawn_kind("shock-wave", Vec3.new(y, 0, 0), params)
    if o then
        o.panel_x, o.panel_y = x, y
        o.element = 0
        o.damage = damage
        o.stamina = bit32.rshift(damage, 16)
        o.alliance, o.flip = from.alliance, from.flip
        o.related1 = from
    end
    return o
end

-- Its row of data.objects.shock_waves.
local function look(me: Object): ShockWave
    local w = data.objects.shock_waves[me:param(1)]
    if w == nil then
        error(string.format("shock wave variant %#x reads past byte_80C6B00", me:param(1)))
    end
    return w
end

-- `sub_80C6CFC`: what it does to the panel it comes onto.
local function mark(x: number, y: number, panel: PanelType?)
    if panel == nil then
        return
    elseif panel == "cracked" then
        field.crack(x, y)
    elseif panel == "broken" then
        field.shatter(x, y)
    else
        field.set_type(x, y, panel)
    end
end

-- Off the field (X's whole pixels at 240, the fraction kept).
local function off_field(me: Object)
    local p = me.pos
    me.pos = Vec3.new(bit32.bor(OFF_FIELD * 0x10000, bit32.band(p.x, 0xFFFF)), p.y, p.z)
end

-- `sub_80C6D1E`: where WaveArm's next wave goes. Once it turned (its
-- second parameter), each next wave takes the same row change, where the
-- row exists. Before that: straight on if an enemy is in the region ahead;
-- else up a row if an enemy is up there ahead, else down a row likewise
-- (the turn is then kept); else straight on.
local function steer(me: Object): (number, number)
    local x, y = me.panel_x + me.facing, me.panel_y
    local turned = me:param(2)
    if turned ~= 0 then
        local dy = ROW_TURN[turned]
        if dy == nil then
            error(string.format("shock wave row change %d reads past dword_80C6DC8", turned))
        end
        local row = y + dy
        if row == 0 or row > 3 then
            return x, y
        end
        return x, row
    end
    local ahead = AHEAD[bit32.bxor(me.alliance, me.flip)][me.panel_x]
    if ahead == nil then
        error(string.format("shock wave on column %d reads past byte_80C6DB8", me.panel_x))
    end
    if panels.random_enemy_panel(me, x, y, ahead) then
        return x, y
    end
    if y - 1 ~= 0 and panels.random_enemy_panel(me, x, y - 1, UP_REGION) then
        me:set_param(2, 1)
        return x, y - 1
    end
    if y + 1 <= 3 and panels.random_enemy_panel(me, x, y + 1, DOWN_REGION) then
        me:set_param(2, 2)
        return x, y + 1
    end
    return x, y
end

-- `sub_80C6C6A` (action 0): when its ticks are up, send the next wave on
-- and linger; out of sight at its animation's end.
local function roll(me: Object)
    if me.phase == 0 then
        me.phase = 1
    end
    local t = me.timer - 1
    me.timer = t
    if t == 0 then
        local variant = me:param(1)
        local x, y
        if variant >= 0xC and variant <= 0xE then
            x, y = steer(me)
        else
            x, y = me.panel_x + me.facing, me.panel_y
        end
        local params = { me:param(1), me:param(2), me:param(3), me:param(4) }
        shock_wave.spawn(me, x, y, params, me.damage + me.stamina * 0x10000)
        me.action = 4
    end
    if bit32.btest(me.sprite.frame_flags, 0x80) then
        off_field(me)
        me.collision.region = 0
    end
end

-- `sub_80C6CBA` (action 4): gone at its animation's end, or (variants 0xC
-- and up) after its second timer.
local function linger(me: Object)
    if me:param(1) < 0xC then
        if not bit32.btest(me.sprite.frame_flags, 0x80) then
            return
        end
    else
        local t = me.timer2 - 1
        me.timer2 = t
        if t ~= 0 then
            return
        end
    end
    off_field(me)
    me.collision.region = 0
    me:set_lifecycle("destroy")
end

-- `sub_80C6C14`: resolve its hits; it ends with the battle or when its
-- panel stops being solid, and stops hitting once it hit something.
local function tick(me: Object)
    local c = me.collision
    c:remove()
    me:hit_spark()
    if battle.over() or not bit32.btest(field.flags(me.panel_x, me.panel_y), SOLID) then
        c.region = 0
        me:set_lifecycle("destroy")
        return
    end
    if c.hit_flags ~= 0 then
        c.region = 0
    end
    local a = me.action
    if a == 0 then
        roll(me)
    elseif a == 4 then
        linger(me)
    else
        error(string.format("shock wave action %#x reads past its table", a))
    end
    c:present()
    me:highlight_collision_panels()
end

-- `sub_80C6B64`: on a solid panel of the field, or not at all.
local function init(me: Object)
    local x, y = me.panel_x, me.panel_y
    if not field.valid(x, y) or not bit32.btest(field.flags(x, y), SOLID) then
        me:free()
        return
    end
    local w = look(me)
    local sprite = me.sprite
    sprite:load(w.sprite)
    sprite.shadow = "with_sprite"
    me.visible = true
    me.anim = w.anim
    me.anim_loaded = w.anim
    sprite:set_animation(w.anim)
    me.timer = w.ticks
    me.timer2 = 4
    sprite.palette = 0
    sprite:set_flip(bit32.bxor(me.alliance, me.flip))
    mark(x, y, w.panel)
    me:set_coordinates_from_panel()
    if not me:create_collision() then
        me:free()
        return
    end
    -- One that marks its panel breaks things too.
    me:setup_collision(if w.panel == nil then 4 else 0x0A, 5, 3)
    local c = me.collision
    c.region = 1
    c.hit_effect = 0xFF
    c:present()
    battle.play_sound(0xA6)
    me:set_lifecycle("update")
end

-- `sub_80C6B40`.
function shock_wave.update(me: Object)
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
end

return shock_wave
