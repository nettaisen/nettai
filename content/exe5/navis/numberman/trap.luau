--!strict
-- NumTrap's trap (EXE5's attack object #0xC3, 0x080DF1E0; EXE5's own), which NumTrap's controller (./controller)
-- sets on a panel during its dimming: a mine (the land mine's sprite in its palette 1) that shows for 41 ticks and
-- then sits out of sight (255 pixels up, as the original hides it). Once the battle goes on it waits for a body of
-- the other side's on its panel: then, with a sound, its press (./press) comes down on four panels around it and it
-- is gone. It goes too when its panel can no longer be stood on, and when the battle is over.
--
-- The press's four panels are a square with the trap's panel in it, drawn among the squares that fit: the trap's
-- panel is its top left, top right, bottom left or bottom right panel, where the square's top left panel is on the
-- field and above the bottom row. (Nothing tests the square's right column, so a trap in the last column can bring
-- the press down half off the field.)

local press = require("./press")

local trap = {}

local SPRITE = asset.sprite("land-mine")
local SOUND = asset.sound("tango-land")
local PALETTE = 1
local SHOWN_TICKS = 40
-- The upper half of its height once it is hidden.
local HIDDEN = 0xFF
-- The panel flags' bit of a panel something can stand on, and the other side's bodies by its side (0x080DF2E0).
local STANDS = 0x10
local ENEMY: { [number]: number } = { [0] = 0x0500_0000, 0x0A00_0000 }
-- Where the press's top left panel may be from the trap's (0x080DF378), in the order they are drawn from.
local SQUARES = { { 0, 0 }, { 0, -1 }, { -1, 0 }, { -1, -1 } }
-- The press's variant: the trap's wait and speed.
local PRESS_VARIANT = 4

-- Its actions.
local SHOWN, ARMED = 0x0, 0x4

-- 0x080DF274: it shows for 41 ticks (the dimming's too), then hides.
local function show(me: Object)
    if me.phase == 0 then
        me.phase = 4
        me.timer = SHOWN_TICKS
        return
    end
    local left = me.timer - 1
    me.timer = int.u16(left)
    if left >= 0 then
        return
    end
    local p = me.pos
    me.pos = Vec3.new(p.x, p.y, int.i32(bit32.bor(bit32.band(p.z, 0xFFFF), HIDDEN * 0x1_0000)))
    me:set_action(ARMED)
end

-- 0x080DF314: the press's top left panel, drawn among the squares that fit.
local function square(me: Object): (number, number)
    local fits: { { number } } = {}
    for _, d in SQUARES do
        local x, y = me.panel_x + d[1], me.panel_y + d[2]
        if y < 3 and field.valid(x, y) then
            table.insert(fits, { x, y })
        end
    end
    if #fits == 0 then
        -- No square fits (never so for a panel of the field: its own square or the one above does): column 0, and
        -- the row the last square tried left in its register, the one above the trap's.
        return 0, me.panel_y - 1
    end
    local pick = fits[battle.rng_positive() % #fits + 1]
    return pick[1], pick[2]
end

-- 0x080DF29C: gone with a panel nothing can stand on; sprung by a body of the other side's on it.
local function armed(me: Object)
    local flags = field.flags(me.panel_x, me.panel_y)
    if bit32.band(flags, STANDS) ~= 0 then
        if bit32.band(flags, ENEMY[me.alliance]) == 0 then
            return
        end
        battle.play_sound(SOUND)
        local x, y = square(me)
        press.spawn(me, x, y, me.element, PRESS_VARIANT, bit32.bor(me.damage, bit32.lshift(me.stamina, 16)))
    end
    me.lifecycle = "destroy"
end

-- 0x080DF204: on its panel, on the ground; the mine's sprite (its first part drawn with it) in its palette, shown.
local function init(me: Object)
    me:set_coordinates_from_panel()
    local p = me.pos
    me.pos = Vec3.new(p.x, p.y, bit32.band(p.z, 0xFFFF))
    local sprite = me.sprite
    sprite:load(SPRITE)
    -- (`sprite_noShadow`: the first part is drawn with the sprite.)
    sprite.shadow = "with_sprite"
    me.visible = true
    me.anim, me.anim_loaded = 0, 0
    sprite:set_animation(0)
    sprite.palette = PALETTE
    me:set_lifecycle("update")
end

-- 0x080DF23E: while it shows it runs dimmed or not; armed, it stands still while dimmed.
local function tick(me: Object)
    if battle.over() then
        me.lifecycle = "destroy"
        return
    end
    local a = me.action
    if a == SHOWN then
        show(me)
    elseif a == ARMED then
        if not battle.dimmed() then
            armed(me)
        end
    else
        error(string.format("NumTrap's trap: action %#x reads past its table (0x080DF26C)", a))
    end
end

trap.kind = new.kind {
    id = "numberman/trap",
    pool = "attack",
    -- 0x080DF1E0
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

-- 0x080DF2E8: a trap of `owner`'s (its controller's) side and facing on (x, y), with the element and the damage
-- word, running while dimmed. (Its position is the spawner's registers: the row, the element and `fraction`, the
-- controller's bonus, which stays its height's lower half.)
function trap.spawn(owner: Object, x: number, y: number, element: number, damage: number, fraction: number): Object?
    local o = battle.spawn(trap.kind, Vec3.new(y, element, fraction))
    if o then
        o.panel_x, o.panel_y = x, y
        o.element = element
        o.related1 = owner
        o.alliance, o.flip = owner.alliance, owner.flip
        o:set_damage_word(damage)
        o.run_while_dimmed = true
    end
    return o
end

return trap
