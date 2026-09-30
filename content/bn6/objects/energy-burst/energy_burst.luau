--!strict
-- The energy burst (attack object #0x11, `sub_80C67F8`): what EnergBom's
-- and MegEnBom's bomb (objects/bomb, kind 2) leaves where it lands. It
-- hits its panel three times, 7 ticks apart (the first at once), with
-- hit modifiers 1, 1 and 3, then shows for 19 more ticks.

local energy_burst = {}

local SPRITE = "14-12"
local SOUND = 0xBB
local HIT_INTERVAL, HITS, LINGER = 7, 3, 0x14
-- The hits' hit modifiers, by hit (`dword_80C68AC`).
local HIT_MODS = { [0] = 1, [1] = 1, [2] = 3 }

-- `sub_80C68B0`: a burst for `owner` on (x, y), with the damage word.
-- (Its position is the spawner's registers, the panel row and element,
-- until its init places it; the game's parameters are whatever the
-- spawner's caller left in a register, and it never reads them.)
function energy_burst.spawn(owner: Object, x: number, y: number, element: number, damage: number, stamina: number): Object?
    local o = battle.spawn_kind("energy-burst", Vec3.new(y, element, 0))
    if o then
        o.panel_x, o.panel_y = x, y
        o.element = element
        o.alliance = owner.alliance
        o.damage = damage
        o.stamina = stamina
        o.related1 = owner
    end
    return o
end

-- `sub_80C6854`: the hits, then the linger.
local function tick(me: Object)
    if me.phase == 0 then
        if me.phase_init == 0 then
            me.phase_init = 4
        end
        local t = me.timer - 1
        me.timer = t
        if t > 0 then
            return
        end
        me.timer = HIT_INTERVAL
        local n = me.timer2
        local hit_mod = HIT_MODS[n]
        if hit_mod == nil then
            error(string.format("energy burst hit %d reads past dword_80C68AC", n))
        end
        battle.hitbox(me, {
            panel_x = me.panel_x,
            panel_y = me.panel_y,
            element = me.element,
            z = 0,
            region = 0x01,
            hit_effect = 0xFF,
            target = 0x05,
            self_type = 0x0A,
            damage = me.damage,
            stamina = me.stamina,
            hit_mod = hit_mod,
        })
        n += 1
        me.timer2 = n
        if n < HITS then
            return
        end
        me.phase, me.phase_init = 4, 0
        me.timer = LINGER
    end
    local t = me.timer - 1
    me.timer = t
    if t <= 0 then
        me:set_lifecycle("destroy")
    end
end

-- `sub_80C681C`.
local function init(me: Object)
    local sprite = me.sprite
    sprite:load(SPRITE)
    sprite.shadow = "with_sprite"
    me.visible = true
    me.anim = 0
    me.anim_loaded = 0xFF
    me:set_coordinates_from_panel()
    battle.play_sound(SOUND)
    me:set_lifecycle("update")
    tick(me)
end

-- `sub_80C67F8`.
function energy_burst.update(me: Object)
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

return energy_burst
