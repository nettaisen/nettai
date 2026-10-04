--!strict
-- CloudMan's cloud (BN5's attack object #0xB3, 0x080DCA10; BN5's own: BN6's #0xB3 is DustMan's junk, on the same
-- skeleton), on a panel behind an enemy: hidden for 80 ticks, it shows (a sound), waits 30, charges 60 (animation
-- 7, a sound), and strikes: a bolt (objects/flying-shot's row 0xB: elec, paralyzing) from 16 pixels ahead of it,
-- 16 up, toward the enemy, running while dimmed; 91 ticks on it goes. The battle's end ends it at once.

local flying_shot = require("../../objects/flying-shot")

local cloud = {}

local SPRITE = asset.sprite("cloudman")
local SOUND_SHOW = asset.sound("panel-poison")
local SOUND_CHARGE = asset.sound("erase-man-2")
local SOUND_BOLT = asset.sound("wave")
local BOLT_ROW = 0xB

-- Enter the action with a timer (its phase byte the entry flag), or count down; true once it has run out.
local function countdown(me: Object, ticks: number): boolean
    if me.phase == 0 then
        me.phase = 4
        me.timer = ticks
        return false
    end
    local left = me.timer - 1
    me.timer = int.u16(left)
    return left <= 0
end

-- Action 0 (0x080DCAA4): 80 ticks hidden; then it shows, with a sound.
local function wait(me: Object)
    if countdown(me, 0x50) then
        me.visible = true
        battle.play_sound(SOUND_SHOW)
        me:set_action(4)
    end
end

-- Action 4 (0x080DCAD6): 30 ticks.
local function hover(me: Object)
    if countdown(me, 0x1E) then
        me:set_action(8)
    end
end

-- Action 8 (0x080DCAFA): its charge (animation 7, a sound), 60 ticks; then the bolt, with its sound.
local function charge(me: Object)
    if me.phase == 0 then
        me.phase = 4
        me.anim = 7
        battle.play_sound(SOUND_CHARGE)
        me.timer = 0x3C
        return
    end
    local left = me.timer - 1
    me.timer = int.u16(left)
    if left > 0 then
        return
    end
    local p = me.pos
    local bolt = flying_shot.launch(me, Vec3.new(p.x + me.facing * 0x10_0000, p.y, 0x10_0000), BOLT_ROW,
        bit32.bor(me.damage, bit32.lshift(me.stamina, 16)))
    if bolt then
        bolt.run_while_dimmed = true
    end
    battle.play_sound(SOUND_BOLT)
    me:set_action(0xC)
end

-- Action 0xC (0x080DCB4A): animation 6, 91 ticks (to -1); then it goes.
local function linger(me: Object)
    if me.phase == 0 then
        me.phase = 4
        me.anim = 6
        me.timer = 0x5A
        return
    end
    local left = me.timer - 1
    me.timer = int.u16(left)
    if left < 0 then
        me:set_lifecycle("destroy")
    end
end

local ACTIONS = { [0] = wait, [4] = hover, [8] = charge, [0xC] = linger }

-- 0x080DCA72: gone at the battle's end; else its action.
local function tick(me: Object)
    if battle.over() then
        me:set_lifecycle("destroy")
        return
    end
    local action = ACTIONS[me.action]
    if not action then
        error(string.format("CloudMan's cloud: action %#x reads past its table (0x080DCA94)", me.action))
    end
    action(me)
end

-- 0x080DCA34: on its panel, on the ground; CloudMan's sprite (animation 6, its shadow), palette 0, facing its way;
-- hidden; then its first update.
local function init(me: Object)
    me:set_coordinates_from_panel()
    local p = me.pos
    me.pos = Vec3.new(p.x, p.y, 0)
    local sprite = me.sprite
    sprite:load(SPRITE)
    sprite.shadow = "ground"
    me.anim, me.anim_loaded = 6, 6
    sprite:set_animation(6)
    sprite.palette = 0
    sprite:set_flip(bit32.bxor(me.alliance, me.flip))
    me:set_lifecycle("update")
    tick(me)
end

cloud.kind = define.kind {
    id = "cloudman/cloud",
    pool = "attack",
    -- 0x080DCA10.
    update = function(me: Object)
        local l = me.lifecycle
        if l == "init" then
            init(me)
        elseif l == "update" then
            tick(me)
        else
            me:destroy()
        end
        me:update_sprite()
    end,
}

-- 0x080DCB6E: a cloud of `owner`'s on (x, y) with `element`, its flip `flip`, the damage word and the owner's
-- side, running while dimmed. (Its position is the spawner's registers, which its init overwrites.)
function cloud.spawn(owner: Object, x: number, y: number, element: number, flip: number, damage: number): Object?
    local o = battle.spawn(cloud.kind, Vec3.zero)
    if o then
        o.panel_x, o.panel_y = x, y
        o.element = element
        o.flip = flip
        o:set_damage_word(damage)
        o.alliance = owner.alliance
        o.run_while_dimmed = true
    end
    return o
end

return cloud
