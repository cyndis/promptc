set PROMPTC_DIR (dirname (status --current-filename))

function fish_prompt
	eval $PROMPTC_DIR/target/release/promptc
end
function fish_title
	eval $PROMPTC_DIR/target/release/promptc --title
end
function fish_right_prompt
	eval $PROMPTC_DIR/target/release/promptc --right
end
