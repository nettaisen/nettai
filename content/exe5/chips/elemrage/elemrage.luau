--!strict
-- EXE5's ElemRage (action 0x56, 0x080EFCD0): EXE5's own. The navi raises its claw (the attachment table's row 0x34,
-- 0c-33, its animation 1) and sends a flame (attack object #0x98, 0x080D99D8) along its row from the panel ahead, at
-- its attach point 20; each flame, when its animation first ends, sends the next one ahead until the record's count
-- (its first parameter, 3) is spent, and stands its time (the second, 30 ticks). On lava, sea, ice, grass or magnet
-- the navi takes that panel's element (0x080EFDE4: fire, aqua, aqua, wood, elec) and palette for the flames, which
-- then also spread to the rows beside and paralyze, and its own panel turns back to normal (the panel changer's row
-- 11). Its roar sounds every 16 ticks; 31 ticks in, 6 more, and it is back to idle.

local attachment = require("../../objects/attachment")
local collision = require("../../rules/collision")
local panel_changer = require("../../objects/panel_changer")
local regions = require("@exelib/regions")
local sparks = require("../../lib/sparks")

local elemrage = {}

-- ---- The flame ------------------------------------------------------------------------------------------------------

local SPRITE = asset.sprite("rage-flame")
-- EXE5's panel types by number (compat/panels.toml, 0x0800BCF0), for the type a flame leaves.
local PANEL_TYPES: { [number]: PanelType } = {
    [0] = "missing",
    [1] = "broken",
    [2] = "normal",
    [3] = "cracked",
    [4] = "poison",
    [5] = "magnet",
    [6] = "grass",
    [7] = "ice",
    [8] = "lava",
    [9] = "holy",
    [10] = "sea",
}

export type FlameState = {
    -- Its count left (Param1, its low seven bits), the spread to the rows beside (Param1's bit 7), its time
    -- (Param2), its X offset (Param3), and its flags (Param4: 1 runs while dimmed; 4 paralyzes, hit modifier 1, its
    -- hit back 12 ticks after one; 8 its self type row 0x25; its high nibble a panel type to leave).
    count: number,
    spread: boolean,
    ticks: number,
    offset: number,
    flags: number,
    -- Its palette (+0x0D), it has hit (+0x0C) and it has sent the next ones (+0x60).
    palette: number,
    hit: boolean,
    sent: boolean,
}

local spawn_flame: (owner: Object, x: number, y: number, element: number, z: number, s: FlameState, damage: number) -> Object?

-- Action 0 (0x080D9B50): when its animation ends: its idle animation, its hit on; the first time, the next flame ahead
-- (and, spreading, the two beside it), its count one less.
local function burst(me: Object)
    if not bit32.btest(me.sprite.frame_flags, 0x80) then
        return
    end
    local s = me.state :: FlameState
    me.anim = 0
    me.collision:set_region(regions.single)
    if not s.sent then
        s.sent = true
        if s.count > 1 then
            s.count -= 1
            local x, y = me.panel_x + me.facing, me.panel_y
            local damage = bit32.bor(me.damage, bit32.lshift(me.stamina, 16))
            local function flame(flags: number): FlameState
                return {
                    count = s.count,
                    spread = false,
                    ticks = s.ticks,
                    offset = s.offset,
                    flags = flags,
                    palette = s.palette,
                    hit = false,
                    sent = false,
                }
            end
            local o = spawn_flame(me, x, y, me.element, me.pos.z, flame(s.flags), damage)
            if o then
                o.related1 = me.related1
            end
            if s.spread then
                -- (The ones beside, above then below, without Param4's bit 2.)
                for _, dy in { -1, 1 } do
                    local q = spawn_flame(me, x, y + dy, me.element, me.pos.z, flame(bit32.band(s.flags, bit32.bnot(4))), damage)
                    if q then
                        q.related1 = me.related1
                    end
                end
            end
        end
    end
    me:set_action(4)
end

-- 0x080D9AA0: its panel's type left at 3 ticks to go; its time out (or its owner gone), it ends; a hit takes its region
-- (a paralyzing one back 12 ticks later, hit modifier 3 then); its action; its panel highlighted while it can hit.
local function tick(me: Object)
    local c = me.collision
    c:remove()
    me:hit_spark()
    local s = me.state :: FlameState
    if me.timer == 3 then
        local kind = bit32.rshift(s.flags, 4)
        if kind ~= 0 then
            local t = PANEL_TYPES[kind]
            if not t then
                error(string.format("ElemRage's flame: panel type %d is past EXE5's types", kind))
            end
            field.set_type(me.panel_x, me.panel_y, t)
        end
    end
    local t = me.timer - 1
    me.timer = int.u16(t)
    local owner = me.related1
    local ended = t == 0 or (owner ~= nil and not owner.active)
    if not ended then
        if c.hit_flags ~= 0 then
            s.hit = true
            me.timer2 = 0
            c.hit_mod_base = 3
            c:set_region(nil)
        end
        ended = battle.over()
    end
    if ended then
        c:set_region(nil)
        me:set_lifecycle("destroy")
        me.visible = false
        return
    end
    if me.action == 0 then
        burst(me)
    elseif me.action ~= 4 then
        error(string.format("ElemRage's flame: action %#x reads past 0x080D9B48", me.action))
    end
    if bit32.btest(s.flags, 4) and s.hit then
        me.timer2 = int.u16(me.timer2 + 1)
        if me.timer2 == 12 then
            c:set_region(regions.single)
        end
    end
    if c.region ~= nil then
        field.highlight(me.panel_x, me.panel_y)
    end
    c:present()
end

-- 0x080D9A08: off the field, gone; else shown on its panel (its X by its offset ahead), its time, its palette, its
-- collision (an attack, or row 0x25, that hits navis: hit modifier 3, or 1 paralyzing), its element's spark, no
-- region yet; the first update at once.
local function init(me: Object)
    if not field.valid(me.panel_x, me.panel_y) then
        me:free()
        return
    end
    local s = me.state :: FlameState
    me.visible = true
    local sprite = me.sprite
    sprite:load(SPRITE)
    sprite.shadow = "with_sprite"
    me.anim, me.anim_loaded = 0, 0xFF
    sprite:set_flip(bit32.bxor(me.alliance, me.flip))
    me:set_coordinates_from_panel()
    local p = me.pos
    me.pos = Vec3.new(int.i32(p.x + int.shl(me.facing * s.offset, 16)), p.y, p.z)
    me.timer = s.ticks
    s.hit, s.sent = false, false
    sprite.palette = s.palette
    if not me:create_collision() then
        me:free()
        return
    end
    local hit_mod = if bit32.btest(s.flags, 4) then 1 else 3
    local is = if bit32.btest(s.flags, 8) then collision.attack_480 else collision.attack
    me:setup_collision(is, collision.hits_navis, hit_mod)
    me.collision:set_hit_effect(sparks.of_element(me.element))
    me.collision:set_region(nil)
    me.collision:present()
    me:set_lifecycle("update")
    tick(me)
end

elemrage.flame = new.kind {
    id = "elemrage/flame",
    pool = "attack",
    state = {
        count = "u8",
        spread = "bool",
        ticks = "u8",
        offset = "i8",
        flags = "u8",
        palette = "u8",
        hit = "bool",
        sent = "bool",
    },
    -- 0x080D99D8: by its lifecycle, then a sprite step (while dimmed too, with Param4's bit 0).
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
            me:free()
            return
        end
        if bit32.btest((me.state :: FlameState).flags, 1) then
            me:update_sprite_while_dimmed()
        else
            me:update_sprite()
        end
    end,
}

-- 0x080D9BE6: a flame on (x, y) with its state, the element, height and damage word, of `owner`'s side and facing,
-- of no owner; running while dimmed with Param4's bit 0. (Its position is the spawner's registers until its init
-- places it: its row, its element and its height.)
spawn_flame = function(owner: Object, x: number, y: number, element: number, z: number, s: FlameState, damage: number): Object?
    local o = battle.spawn(elemrage.flame, Vec3.new(y, element, z))
    if not o then
        return nil
    end
    o.panel_x, o.panel_y = x, y
    o.element = element
    o:set_damage_word(damage)
    o.alliance, o.flip = owner.alliance, owner.flip
    o.related1 = nil
    local t = o.state :: FlameState
    t.count, t.spread, t.ticks, t.offset, t.flags, t.palette = s.count, s.spread, s.ticks, s.offset, s.flags, s.palette
    if bit32.btest(s.flags, 1) then
        o.run_while_dimmed = true
    end
    return o
end

-- ---- The action -----------------------------------------------------------------------------------------------------

export type State = {
    -- Ticks into the action (AIAttackVars+0x10) and the roar's count (+0x12).
    timer: number,
    roar: number,
    -- The flames' palette (+0x30) and whether the navi took a panel's element (+0x0F bit 2).
    palette: number,
    raged: boolean,
}

-- The panel types that give an element (0x080EFDE4): the element, the flames' palette.
local RAGES: { [string]: { element: number, palette: number } } = {
    lava = { element = 1, palette = 0 },
    sea = { element = 2, palette = 1 },
    ice = { element = 2, palette = 1 },
    grass = { element = 4, palette = 2 },
    magnet = { element = 3, palette = 3 },
}
local NONE = { element = 0, palette = 4 }
local CLAW = attachment.look { sprite = asset.sprite("rage-claw"), palette = 0, attach_point = 0x16 }
-- (The flame and the claw MegaMan's B↓FireAm fires too: navis/megaman/weapons/fire_arm.)
elemrage.spawn_flame = spawn_flame
elemrage.claw = CLAW
local ROAR, RAGE_ROAR = asset.sound("roar"), asset.sound("rage-roar")
-- The user's panel back to normal (the panel changer's row 11: its own panel, 30 ticks, flickering).
local RESTORE = panel_changer.change {
    panel = "normal",
    panels = "own",
    flicker = true,
    ticks = 30,
    sides = { [0] = { require = 0, forbid = 0x8020 }, [1] = { require = 0x20, forbid = 0x8000 } },
    tick_sound = panel_changer.TICK_SOUND,
    change_sound = panel_changer.CHANGE_SOUND,
}
local ANIM, ARM_ANIM, ATTACH_POINT = 10, 1, 20
local LAST_TICK, RECOVER_TICKS = 30, 5

-- ElemRage's action: its flames' count, time and flags (the record's parameters).
function elemrage.action(count: number, ticks: number, flags: number): Action
    -- 0x080EFCEC: the rage by the panel; the flame out; the claw raised; the roar every 16 ticks; on after the 31st.
    local function fire(me: Object, s: State)
        if me.step_init == 0 then
            local p = field.panel(me.panel_x, me.panel_y)
            local rage = (p and RAGES[p.kind]) or NONE
            me.attack_element = rage.element
            s.palette = rage.palette
            local spread, flame_flags = false, flags
            s.raged = rage.element ~= 0
            if s.raged then
                flame_flags = bit32.bor(flame_flags, 4)
                spread = true
                panel_changer.spawn(me, me.panel_x, me.panel_y, RESTORE, nil)
            end
            -- (The damage word takes the Atk+ bonus, AIAttackVars+8.)
            local damage = me.attack_damage + me.hit_param * 0x10000 + me.extra
            -- (`sub_8018842` reads the attach point unflipped: the flame turns it its own way.)
            local fx, dz = me:attach_point(ATTACH_POINT)
            local dx = fx * me.facing
            spawn_flame(me, me.panel_x + me.facing, me.panel_y, me.attack_element, int.shl(dz, 16), {
                count = count,
                spread = spread,
                ticks = ticks,
                offset = int.i8(dx),
                flags = flame_flags,
                palette = s.palette,
                hit = false,
                sent = false,
            }, damage)
            me.anim = ANIM
            me:open_counter_window()
            attachment.attach(me, CLAW, "overlay", { anim = ARM_ANIM })
            me:set_status("using_action", true)
            s.timer = 0
            me.step_init = 4
        end
        s.roar = bit32.band(s.roar + 1, 15)
        if s.roar == 1 then
            battle.play_sound(if s.raged then RAGE_ROAR else ROAR)
        end
        s.timer += 1
        if s.timer >= LAST_TICK then
            me.step, me.step_init = 4, 0
        end
    end

    -- 0x080EFDFC: 6 ticks, then back to idle.
    local function recover(me: Object, s: State)
        if me.step_init == 0 then
            s.timer = RECOVER_TICKS
            me.step_init = 4
        end
        local t = s.timer - 1
        s.timer = int.u16(t)
        if t < 0 then
            me.related1 = nil
            me.overlay = nil
            me:exit_attack()
        end
    end

    return new.action {
        state = { timer = "u16", roar = "u16", palette = "u8", raged = "bool" },
        -- 0x080EFCD0.
        update = function(me: Object, s: State)
            if me.step == 0 then
                fire(me, s)
            elseif me.step == 4 then
                recover(me, s)
            else
                error(string.format("ElemRage phase %#x reads past 0x080EFCE4", me.step))
            end
        end,
    }
end

return elemrage
