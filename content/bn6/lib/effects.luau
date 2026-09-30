--!strict
-- One-shot effects many kinds show (`battle.effect(pos, effects.explosion)`):
-- a sprite's animation that plays once. Effects one owner shows live with
-- it.

local effects = {}

-- An explosion: a blast landing, something breaking (effect 0x00).
effects.explosion = define.effect { sprite = asset.sprite("explosion"), anim = 0, palette = 0 }

-- A splash of bubbles: something landing in a hole (effect 0x01).
effects.splash = define.effect { sprite = asset.sprite("rising-bubble"), anim = 0, palette = 0 }

-- A puff of smoke: a thrown thing gone when the battle ends, a field
-- object a chip removes (effect 0x14).
effects.puff = define.effect { sprite = asset.sprite("puff"), anim = 2, palette = 0 }

return effects
