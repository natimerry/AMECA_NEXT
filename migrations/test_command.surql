SELECT VALUE array::distinct(->sent->message
<-sent_in_channel 
    <-(channel // gets all the channels 
        WHERE <-has_channel<- 
            (guild WHERE guild_id == $guild_id)) // from the same server as the original interaction was created in 

        .name) // queries the channel name
FROM members WHERE name == $user ; // only for the particular user