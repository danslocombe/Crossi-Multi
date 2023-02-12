
export function create_audio_manager()
{
    return {

        sounds_enabled : true,

        // Browsers will block audio of pages that try and play sounds before there have been user inputs
        // So we don't try and play any before there is a user input
        webpage_has_inputs : false,

        play : function(sound)
        {
            if (this.sounds_enabled && this.webpage_has_inputs)
            {
                sound.play();
            }
        }
    };
}