--!strict
-- NapalmSoul's charged Fire chips (BN5's action 0x4B, 0x080EF2A2, which a charged chip starts in NapalmSoul in place
-- of its own: 0x08010442): MegaMan glows (effect 0x26: sprite 14-1D on him for 20 ticks) and on the eleventh tick
-- lobs a napalm bomb (chips/napalmmn/bomb, variant 2) with the chip's damage at the first enemy body he finds two
-- columns ahead and on (each column his row first; none on the field: four panels ahead in his row, 0x0801265C).
-- 21 ticks, then 11 of recovery.

local bomb = require("../../../../chips/napalmmn/bomb")

local charged_chip = {}

export type State = {
    -- AIAttackVars +0x10: the step's ticks (counted up, then down).
    timer: number,
}

export type FlashState = {
    -- Param1 to Param4: its sprite's category and index, its animation and whether it has a shadow.
    sprite: SpriteAsset,
    anim: number,
    shadow: boolean,
}

local THROW_ANIM, RECOVER_ANIM = 18, 19
-- The glow's sprite (its parameters 0x141D: 14-1D, animation 0, no shadow).
local FLASH = asset.sprite("bn5:napalmsoul-flash")
local THROW_AT, THROW_TICKS, RECOVER_TICKS = 10, 21, 10
-- The bomb leaves 10 pixels ahead of him and 48 up, of variant 2.
local AHEAD, UP = 10 * 0x1_0000, 48 * 0x1_0000
local VARIANT = 2
-- An enemy's body, by side (0x080126D0).
local ENEMY_BODY = { [0] = 0x0400_0000, [1] = 0x0800_0000 }
-- The rows a column is searched in, by his row (0x080126D8).
local ROWS = { { 1, 2, 3 }, { 2, 1, 3 }, { 3, 1, 2 } }

-- ---- The glow (effect 0x26) ---------------------------------------------------------------------------------------

-- BN5's effect object 0x26 (0x080E330C, BN6's `sub_80E3150`, the same code): a sprite (its parameters') on its owner
-- for 20 ticks; then it goes.
charged_chip.flash = define.kind {
    id = "bn5:napalmsoul/flash",
    pool = "effect",
    state = { sprite = "sprite", anim = "u8", shadow = "bool" },
    update = function(me: Object)
        local l = me.lifecycle
        if l == "init" then
            -- 0x080E3330: its sprite (`sub_801BE2A`), shown.
            local s = me.state :: FlashState
            local sprite = me.sprite
            sprite:load(s.sprite)
            sprite.shadow = if s.shadow then "ground" else "with_sprite"
            me.anim, me.anim_loaded = s.anim, 0xFF
            sprite:set_flip(bit32.bxor(me.alliance, me.flip))
            me.visible = true
            me:set_lifecycle("update")
        end
        if me.lifecycle == "update" then
            -- 0x080E334A: 20 ticks; it keeps to its owner.
            if me.phase_init == 0 then
                me.timer = 20
                me.phase_init = 4
            end
            local t = me.timer - 1
            me.timer = bit32.band(t, 0xFFFF)
            if t <= 0 then
                me.visible = false
                me:set_lifecycle("destroy")
            end
            me.pos = (me.related1 :: Object).pos
        elseif me.lifecycle == "destroy" then
            me:free()
            return
        end
        me:update_sprite()
    end,
}

-- `sub_80E31C0`'s counterpart (0x080E337C): the glow on `owner`, of its side and facing.
local function flash(owner: Object)
    local o = battle.spawn(charged_chip.flash)
    if o then
        local s = o.state :: FlashState
        s.sprite, s.anim, s.shadow = FLASH, 0, false
        o.related1 = owner
        o.alliance, o.flip = owner.alliance, owner.flip
    end
end

-- ---- The throw ----------------------------------------------------------------------------------------------------

-- 0x0801265C: the panel of the first enemy body two columns ahead and on (in each column his row first), else the
-- panel four ahead in his row once the columns run off the field.
local function target(me: Object): (number, number)
    local front = me.facing
    local x = me.panel_x + 2 * front
    local rows = ROWS[me.panel_y]
    local mask = ENEMY_BODY[me.alliance]
    while true do
        for _, y in rows do
            if not field.valid(x, y) then
                return me.panel_x + 4 * front, me.panel_y
            end
            if bit32.btest(field.flags(x, y), mask) then
                return x, y
            end
        end
        x += front
    end
end

-- 0x080EF2BC: the counter window open, the glow; the bomb on the eleventh tick; 21 ticks.
local function throw(me: Object, s: State)
    if me.step_init == 0 then
        me:open_counter_window()
        me:set_status("using_action", true)
        me:set_animation(THROW_ANIM)
        flash(me)
        s.timer = 0
        me.step_init = 4
    end
    if s.timer == THROW_AT then
        local x, y = target(me)
        local p = me.pos
        bomb.lob(me, Vec3.new(int.i32(p.x + me.facing * AHEAD), p.y, int.i32(p.z + UP)), {
            variant = VARIANT,
            target_x = x,
            target_y = y,
            damage = int.u32(me.attack_damage + me.hit_param * 0x10000 + me.extra),
        })
    end
    s.timer += 1
    if s.timer >= THROW_TICKS then
        me.step, me.step_init = 4, 0
    end
end

-- 0x080EF334: 11 ticks; then back to idle.
local function recover(me: Object, s: State)
    if me.step_init == 0 then
        me:set_animation(RECOVER_ANIM)
        s.timer = RECOVER_TICKS
        me.step_init = 4
    end
    local t = s.timer - 1
    s.timer = bit32.band(t, 0xFFFF)
    if t < 0 then
        me:exit_attack()
    end
end

charged_chip.action = define.action {
    id = "bn5:napalmsoul/charged-chip",
    state = { timer = "u16" },
    -- 0x080EF2A2
    update = function(me: Object, s: State)
        local step = me.step
        if step == 0 then
            throw(me, s)
        elseif step == 4 then
            recover(me, s)
        else
            error(string.format("NapalmSoul's charged chip's step %#x reads past its table (0x080EF2B4)", step))
        end
    end,
}

return charged_chip
