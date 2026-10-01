--!strict
-- One-shot effects many kinds show (`battle.effect(pos, effects.explosion)`):
-- a sprite's animation that plays once. Effects one owner shows live with
-- it.

local effects = {}

-- An explosion: a blast landing, something breaking (effect 0x00).
effects.explosion = define.effect { sprite = asset.sprite("explosion"), anim = 0, palette = 0 }

-- A splash of bubbles: something landing in a hole (effect 0x01).
effects.splash = define.effect { sprite = asset.sprite("rising-bubble"), anim = 0, palette = 0 }

-- Dust: a rock breaking (effect 0x02).
effects.dust = define.effect { sprite = asset.sprite("rising-bubble"), anim = 1, palette = 0 }

-- A puff of smoke: a thrown thing gone when the battle ends, a field
-- object a chip removes (effect 0x14).
effects.puff = define.effect { sprite = asset.sprite("puff"), anim = 2, palette = 0 }

-- A fiery burst: a bullet's (objects/bullet) and TankCan's shell's hit
-- (effect 0x22).
effects.shell_burst = define.effect { sprite = asset.sprite("shell-burst"), anim = 0, palette = 0 }

-- A burst: a meteor or a fist landing, a snake striking (effect 0x05).
effects.burst = define.effect { sprite = asset.sprite("burst"), anim = 0, palette = 0 }

-- A flash: a gun's shot landing, MegaMan's volley in DblHero, a bullet's
-- burst (objects/bullet's rows 0 to 3) (effect 0x21).
effects.flash = define.effect { sprite = asset.sprite("reflected-shot"), anim = 0, palette = 0 }

return effects
