--!strict
-- The plus chips' sparkle, as BN5 and BN6 share it (BN6's `loc_80104B6`, shared with `sub_80104E0`; BN5's
-- 0x0800E198's): a rising bubble (@exelib/rising-bubble, the game's kind) by the navi's panel, on the enemy's side of
-- it, 4 pixels toward the enemy's side and 48 up; and its sound.

local rising_bubble = require("../rising-bubble/rising_bubble")

local sparkle = {}

local TOWARD, HEIGHT = 4, 48

-- A game's sparkle: its bubble's kind and its sound.
function sparkle.make(bubble: Kind, sound: Sound): (user: Object) -> ()
    return function(user: Object)
        local x, y = field.center(user.panel_x, user.panel_y)
        local toward = 1 - 2 * user.alliance
        rising_bubble.spawn(bubble, Vec3.new(x + int.shl(toward * TOWARD, 16), y, int.shl(HEIGHT, 16)), 0)
        battle.play_sound(sound)
    end
end

return sparkle
