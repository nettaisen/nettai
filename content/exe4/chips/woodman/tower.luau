--!strict
-- WoodMan's wood towers (attack object #0x07, 0x080CDEB8: EXE4's own; EXE6's #7 is the volcanos' eruption), which he
-- raises on panels of the other side: unseen, its panel highlighted every other four ticks for 31; then, on a solid
-- panel with no body or blocker (else it goes), it rises (sheet 0c-0b, the last of each set with its sound), a thrown
-- body hitting navis on its panel (the wood spark; a hit takes its region), for 11 ticks and 21 more, sinks
-- (animation 1, 10 ticks) and goes.

local collision = require("../../rules/collision")
local regions = require("@exelib/regions")
local sparks = require("../../lib/sparks")

local tower = {}

type State = {
    -- Its hit modifier (its spawner's r7, +0x60), and it rises with its sound (Param1 0: the last of its set).
    hit_mod: number,
    sounds: boolean,
}

local SPRITE = asset.sprite("wood-tower")
local SOUND = asset.sound("wood-tower")
local SOLID = 0x10
-- Bodies and blockers on a panel (0x080CDF82).
local BLOCKED = 0x0384_0000
local WAIT, UP = 0x0, 0x4

-- Enter the phase with a timer, or count down; true once it has run out past 0 (`bge`).
local function countdown(me: Object, ticks: number): boolean
    if me.phase_init == 0 then
        me.phase_init = 4
        me.timer = ticks
    end
    local left = me.timer - 1
    me.timer = int.u16(left)
    return left < 0
end

-- Action 0 (0x080CDF5C): 31 ticks, its panel highlighted while the timer's bit 2 is set; then, on a solid panel with
-- no body or blocker, up; else it goes.
local function wait(me: Object)
    if not countdown(me, 30) then
        if bit32.btest(me.timer, 4) then
            field.highlight(me.panel_x, me.panel_y)
        end
        return
    end
    local f = field.flags(me.panel_x, me.panel_y)
    if not bit32.btest(f, SOLID) or bit32.btest(f, BLOCKED) then
        me:set_lifecycle("destroy")
    else
        me:set_action(UP)
    end
end

-- Action 4 (0x080CDFAC): phase 0 (0x080CDFCC) shown, rising (animation 0 from its start), its region, its sound if
-- it is the set's last, 11 ticks; phase 4 (0x080CE00E) 21 ticks; phase 8 (0x080CE02C) sinking (animation 1), 10
-- ticks; then its region off, gone.
local function up(me: Object)
    if me.phase == 0 then
        if me.phase_init == 0 then
            me.visible = true
            me.anim, me.anim_loaded = 0, 0xFF
            me.collision:set_region(regions.single)
            if (me.state :: State).sounds then
                battle.play_sound(SOUND)
            end
        end
        if countdown(me, 10) then
            me.phase, me.phase_init = 4, 0
        end
    elseif me.phase == 4 then
        if countdown(me, 20) then
            me.phase, me.phase_init = 8, 0
        end
    elseif me.phase == 8 then
        if me.phase_init == 0 then
            me.anim = 1
        end
        if countdown(me, 9) then
            me.collision:set_region(nil)
            me:set_lifecycle("destroy")
        end
    else
        error(string.format("WoodMan's tower's phase %#x reads past its table (0x080CDFC0)", me.phase))
    end
end

-- 0x080CDF2C: its hits resolve (a hit takes its region); its action; its body there again.
local function tick(me: Object)
    local c = me.collision
    c:remove()
    if c.hit_flags ~= 0 then
        c:set_region(nil)
    end
    if me.action == WAIT then
        wait(me)
    elseif me.action == UP then
        up(me)
    else
        error(string.format("WoodMan's tower's action %#x reads past its table (0x080CDF54)", me.action))
    end
    c:present()
end

-- 0x080CDEDC: its sheet (0c-0b, no shadow), unseen, in animation 0; on its panel's center; a thrown body hitting
-- navis (its hit modifier, the wood spark), no region yet; then its first tick.
local function init(me: Object)
    local s = me.state :: State
    local sprite = me.sprite
    sprite:load(SPRITE)
    sprite.shadow = "with_sprite"
    me.anim, me.anim_loaded = 0, 0xFF
    me:set_coordinates_from_panel()
    if not me:create_collision() then
        me:free()
        return
    end
    me:setup_collision(collision.thrown, collision.hits_navis, s.hit_mod)
    local c = me.collision
    c:set_hit_effect(sparks.wood)
    c:set_region(nil)
    c:present()
    me:set_lifecycle("update")
    tick(me)
end

tower.kind = new.kind {
    id = "woodman/tower",
    pool = "attack",
    state = { hit_mod = "u8", sounds = "bool" },
    -- 0x080CDEB8: by its state, then its sprite.
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

-- 0x080CE074: a tower on (x, y) of WoodMan's side, his, with `element` and his damage word, hit modifier 1, running
-- while dimmed; rising with its sound if `sounds`. (Its position is the spawner's registers: the row, the element and
-- 0.)
function tower.spawn(woodman: Object, x: number, y: number, element: number, sounds: boolean): Object?
    local o = battle.spawn(tower.kind, Vec3.new(y, element, 0))
    if not o then
        return nil
    end
    o.panel_x, o.panel_y = x, y
    o.element = element
    o.alliance = woodman.alliance
    o:set_damage_word(bit32.bor(woodman.damage, bit32.lshift(woodman.stamina, 16)))
    o.related1 = woodman
    o.run_while_dimmed = true
    local s = o.state :: State
    s.hit_mod, s.sounds = 1, sounds
    return o
end

return tower
